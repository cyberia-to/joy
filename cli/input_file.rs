//! Canonical native witness files; secret material never enters diagnostics.
use serde::{Deserialize, Deserializer};
use std::{fs, io::Read, path::Path};
use trident::runtime::ProgramInput;

const MODULUS: u64 = 18_446_744_069_414_584_321;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_WORDS: usize = 65_536;
const INVALID: &str =
    "invalid joy input file: require schema_version 1 and canonical public/secret arrays";

struct Word(u64);
impl<'de> Deserialize<'de> for Word {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = Word;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a canonical field integer or unsigned decimal string")
            }
            fn visit_u64<E: serde::de::Error>(self, n: u64) -> Result<Word, E> {
                if n < MODULUS {
                    Ok(Word(n))
                } else {
                    Err(E::custom("noncanonical field"))
                }
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Word, E> {
                if s.is_empty()
                    || s.len() > 20
                    || !s.bytes().all(|b| b.is_ascii_digit())
                    || (s.len() > 1 && s.starts_with('0'))
                {
                    return Err(E::custom("noncanonical decimal field"));
                }
                self.visit_u64(s.parse().map_err(|_| E::custom("invalid field"))?)
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn words<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Word>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<Word>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a bounded field array")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut a: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = Vec::new();
            while let Some(word) = a.next_element()? {
                if result.len() == MAX_WORDS {
                    return Err(serde::de::Error::custom("field count limit"));
                }
                result.push(word);
            }
            Ok(result)
        }
    }
    d.deserialize_seq(Visitor)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    #[serde(deserialize_with = "words")]
    public: Vec<Word>,
    #[serde(deserialize_with = "words")]
    secret: Vec<Word>,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    let before = fs::symlink_metadata(path).map_err(|_| "cannot inspect input file")?;
    if !before.is_file() || before.len() > MAX_BYTES as u64 {
        return Err("input must be a regular file of at most 4 MiB, without symbolic links".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    // O_NONBLOCK | O_NOFOLLOW: protect against replacement by a FIFO or link.
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
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let file = options.open(path).map_err(|_| "cannot open input file")?;
    let after = file
        .metadata()
        .map_err(|_| "cannot inspect opened input file")?;
    if !after.is_file() || after.len() > MAX_BYTES as u64 {
        return Err("input file changed or exceeds size limit".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err("input file changed while opening".into());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if before.file_attributes() & 0x400 != 0 || after.file_attributes() & 0x400 != 0 {
            return Err("input file cannot be a reparse point".into());
        }
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read input file")?;
    if bytes.len() > MAX_BYTES {
        return Err("input file exceeds 4 MiB limit".into());
    }
    Ok(bytes)
}

pub(crate) fn load(path: &Path) -> Result<ProgramInput, String> {
    let document: Document = serde_json::from_slice(&read(path)?).map_err(|_| INVALID)?;
    if document.schema_version != 1 || document.public.len() + document.secret.len() > MAX_WORDS {
        return Err(INVALID.into());
    }
    Ok(ProgramInput {
        public: document.public.into_iter().map(|w| w.0).collect(),
        secret: document.secret.into_iter().map(|w| w.0).collect(),
        digests: Vec::new(),
    })
}

/// File and explicit inputs are mutually exclusive, even when arrays are empty.
pub(crate) fn resolve(
    public: &Option<Vec<u64>>,
    secret: &Option<Vec<u64>>,
    path: Option<&Path>,
) -> Result<ProgramInput, String> {
    if let Some(path) = path {
        if public.is_some() || secret.is_some() {
            return Err("--input-file conflicts with --input-values and --secret".into());
        }
        return load(path);
    }
    let public = public.clone().unwrap_or_default();
    let secret = secret.clone().unwrap_or_default();
    if public.len().saturating_add(secret.len()) > MAX_WORDS
        || public.iter().chain(&secret).any(|&v| v >= MODULUS)
    {
        return Err("input requires at most 65536 canonical field words".into());
    }
    Ok(ProgramInput {
        public,
        secret,
        digests: Vec::new(),
    })
}
