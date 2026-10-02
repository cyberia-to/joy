//! Allocation refusal is isolated from the other test executables.
use joy_rs::structured::certificate::transport::{Limits, Reader, Writer};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    io::{self, Read, Write},
};

struct Allocator;
thread_local! { static REJECT: Cell<Option<usize>> = const { Cell::new(None) }; }
fn reject() -> bool {
    REJECT.with(|counter| match counter.get() {
        None => false,
        Some(0) => true,
        Some(n) => {
            counter.set(Some(n - 1));
            false
        }
    })
}
// SAFETY: successful operations delegate unchanged layouts/pointers to System.
// Refusal returns null as GlobalAlloc specifies; deallocation is never refused.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if reject() {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(layout) }
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if reject() {
            std::ptr::null_mut()
        } else {
            unsafe { System.realloc(ptr, layout, size) }
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn under_refusal<T>(after: usize, f: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            REJECT.with(|c| c.set(None));
        }
    }
    REJECT.with(|c| c.set(Some(after)));
    let _reset = Reset;
    f()
}

#[test]
fn payload_allocation_failure_returns_error_without_allocating_its_message() {
    let caps = Limits {
        wire_bytes: 1 << 20,
        decoded_bytes: 1 << 20,
        frames: 100,
    };
    for after in 0..2 {
        let result = under_refusal(after, || Writer::new(io::sink(), [0; 32], caps));
        assert!(matches!(result, Err(e) if e.kind() == io::ErrorKind::OutOfMemory));
    }
    let mut writer = Writer::new(Vec::new(), [0; 32], caps).unwrap();
    writer.write_all(&[0; 1024]).unwrap();
    let bytes = writer.finish().unwrap().0;
    for after in 0..2 {
        let mut reader = Reader::new(bytes.as_slice(), [0; 32], caps).unwrap();
        let mut output = [0; 1024];
        let result = under_refusal(after, || reader.read_exact(&mut output));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::OutOfMemory);
        assert!(reader.finish().is_err());
    }
    let mut writer = Writer::new(io::sink(), [0; 32], caps).unwrap();
    // Once initialized, writer payload buffering and compression reuse storage.
    let result = under_refusal(0, || {
        writer.write_all(&[7; 65_536])?;
        writer.finish()
    });
    assert!(result.is_ok());
}
