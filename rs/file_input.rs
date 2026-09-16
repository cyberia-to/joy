//! Bounded artifact files; never block on a named pipe during header detection.
use std::{fs, io::Read, path::Path};

pub(crate) fn open(path: &Path, limit: usize) -> Result<fs::File, String> {
    let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !before.is_file() {
        return Err("artifact input must be a regular file, not a link or stream".into());
    }
    if before.len() > limit as u64 {
        return Err("artifact file size limit".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    // O_NONBLOCK | O_NOFOLLOW from the platform fcntl ABI. These flags prevent
    // an admitted regular path replaced by a FIFO/link from blocking on open.
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x4 | 0x100);
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000);
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x8000);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    let after = file.metadata().map_err(|e| e.to_string())?;
    if !after.is_file() || after.len() > limit as u64 {
        return Err("artifact input changed or exceeds file size limit".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err("artifact input changed during open".into());
        }
    }
    Ok(file)
}

pub(crate) fn read(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    open(path, limit)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("artifact file size limit".into());
    }
    Ok(bytes)
}

pub(crate) fn has_header(path: &Path, magic: &[u8], limit: usize) -> bool {
    let Ok(mut file) = open(path, limit) else {
        return false;
    };
    let mut header = [0; 8];
    file.read_exact(&mut header).is_ok() && header == magic
}

/// Read a raw formula or JSON bundle through the same bounded regular-file
/// admission as artifacts, including when verification falls back to execution.
pub fn read_program_text(path: &Path) -> Result<String, String> {
    String::from_utf8(read(path, 64 * 1024 * 1024)?).map_err(|_| "program file is not UTF-8".into())
}
