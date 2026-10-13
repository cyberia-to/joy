//! Memory available to a prover, for choosing the wrapped profile.
//!
//! The wrap chain's prover (zheng profile 6) peaks near 15 GB
//! ([`WRAP_PEAK_BYTES`], measured); `joy prove --succinct` wraps by default
//! only when at least that much memory is available, else it proves the
//! recursive profile 5 and says so.
//!
//! Available memory: `JOY_AVAILABLE_MEMORY` (bytes) if set; on macOS the
//! free, inactive and speculative pages of `vm_stat` (pages the
//! kernel hands out without swapping); on Linux `MemAvailable` from
//! `/proc/meminfo`. `None` when it cannot be read.

use std::process::Command;

/// Memory a wrap prove needs resident (IVC + three wrap levels, the
/// prover's chain keys included), with a margin: `joy prove loop.tri
/// --succinct --wrap` peaks at 14.6 GB max RSS on Metal and 13.2 GB on the
/// CPU backend (zheng #61, `audit/wrap-fastverify-2026-10.md` §9); this
/// is that peak plus 15 % (16.8 GB), rounded up to 16 GiB (17.2 GB).
pub const WRAP_PEAK_BYTES: u64 = 16 << 30;

/// Bytes of memory available to a new process, if known.
pub fn available_bytes() -> Option<u64> {
    if let Ok(v) = std::env::var("JOY_AVAILABLE_MEMORY") {
        return v.trim().parse().ok();
    }
    if cfg!(target_os = "macos") {
        let out = Command::new("vm_stat").output().ok()?;
        parse_vm_stat(&String::from_utf8_lossy(&out.stdout))
    } else {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        parse_meminfo(&text)
    }
}

fn parse_vm_stat(text: &str) -> Option<u64> {
    let page: u64 = text
        .lines()
        .next()?
        .split("page size of ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let mut pages = 0u64;
    for line in text.lines() {
        let Some((name, value)) = line.split_once(':') else { continue };
        if matches!(name.trim(), "Pages free" | "Pages inactive" | "Pages speculative") {
            pages += value.trim().trim_end_matches('.').parse::<u64>().ok()?;
        }
    }
    Some(pages * page)
}

fn parse_meminfo(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemAvailable:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// Whether a wrap prove fits: available memory known and ≥ the peak.
pub fn wrap_fits() -> (bool, Option<u64>) {
    let a = available_bytes();
    (a.is_some_and(|a| a >= WRAP_PEAK_BYTES), a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_stat_and_meminfo_parse() {
        let vm = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages free:                              100.\nPages active:                            5.\nPages inactive:                          20.\nPages speculative:                        3.\nPages purgeable:                          2.\nno colon here\n";
        assert_eq!(parse_vm_stat(vm), Some(123 * 16384));
        assert_eq!(parse_meminfo("MemTotal: 10 kB\nMemAvailable:   2048 kB\n"), Some(2048 * 1024));
        assert_eq!(parse_meminfo("MemTotal: 10 kB\n"), None);
    }
}
