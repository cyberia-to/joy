//! Verifying keys of zheng profiles 5 and 6, persisted between processes.
//!
//! A fresh process verifying a recursive (5) or wrapped (6) envelope needs
//! the IVC circuit key (derived in a few hundred ms) and, for profile 6,
//! the final wrap key (derived over zheng's pinned roots in seconds). Joy
//! keeps their layouts as a zheng key bundle (`zheng::envelope::keys`) in
//! a cache directory and installs it before decoding; zheng accepts a
//! bundle only if its digest is the one zheng pins, so a stale or tampered
//! file is refused (and rewritten) — the cache never changes what verifies.
//!
//! Directory: `JOY_CACHE_DIR`, else `$XDG_CACHE_HOME/joy`, else
//! `~/Library/Caches/joy` (macOS) or `~/.cache/joy`; files
//! `keys/zheng-p<profile>-<pinned digest prefix>.keys`.

use std::path::PathBuf;
use zheng::envelope::keys;
pub use zheng::envelope::Profile;

/// The cached profile an envelope's profile byte names (5 or 6).
pub fn profile_of(byte: u8) -> Option<Profile> {
    match byte {
        5 => Some(Profile::Recursive),
        6 => Some(Profile::Wrapped),
        _ => None,
    }
}

/// The cache directory, if one can be named.
pub fn cache_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("JOY_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(d).join("joy"));
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Caches/joy")
    } else {
        home.join(".cache/joy")
    })
}

/// The bundle file of `profile` (5 or 6), named by zheng's pinned digest.
pub fn key_file(profile: Profile) -> Option<PathBuf> {
    let digest = keys::pinned(profile)?;
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    Some(cache_dir()?.join("keys").join(format!("zheng-p{}-{hex}.keys", profile as u8)))
}

/// Where the keys of a verification came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySource {
    /// Already in this process (the prover's, or installed before).
    Process,
    /// Installed from the cache file.
    Cache(PathBuf),
    /// Not cached: the verifier derives them.
    Derive,
}

/// Install the cached keys of `profile` if this process lacks them.
pub fn load(profile: Profile) -> KeySource {
    if keys::cached(profile) {
        return KeySource::Process;
    }
    let Some(path) = key_file(profile) else {
        return KeySource::Derive;
    };
    match std::fs::read(&path) {
        Ok(bytes) if keys::install(&bytes) == Ok(profile) => KeySource::Cache(path),
        _ => KeySource::Derive,
    }
}

/// Write the keys this process holds for `profile` to the cache (atomic:
/// a temporary file renamed); the file's path.
pub fn store(profile: Profile) -> Result<PathBuf, String> {
    let path = key_file(profile).ok_or("no cache directory")?;
    let bundle = keys::export(profile)?;
    let dir = path.parent().ok_or("cache path")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = dir.join(format!(".{}.{}", path.file_name().and_then(|n| n.to_str()).unwrap_or("keys"), std::process::id()));
    std::fs::write(&tmp, &bundle).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}
