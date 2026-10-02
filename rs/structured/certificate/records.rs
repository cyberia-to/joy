use std::io::{Read, Write};
use zheng::execution::disclosed::stream::CacheHandle;

#[derive(Clone, Copy, Debug)]
pub(super) enum Record {
    Atom(u64),
    Pair {
        left: u32,
        right: u32,
    },
    Reset {
        epoch: u64,
        nodes: u32,
    },
    Enter {
        object: u32,
        formula: u32,
    },
    Finish {
        result: u32,
        cache_slot: Option<u32>,
    },
    Reuse(CacheHandle),
}

impl Record {
    pub(super) fn write(&self, out: &mut impl Write) -> Result<(), String> {
        let mut bytes = [0; 17];
        let n = match *self {
            Self::Atom(v) => {
                bytes[1..9].copy_from_slice(&v.to_le_bytes());
                9
            }
            Self::Pair { left, right } => {
                bytes[0] = 1;
                put_pair(&mut bytes, left, right);
                9
            }
            Self::Reset { epoch, nodes } => {
                bytes[0] = 2;
                bytes[1..9].copy_from_slice(&epoch.to_le_bytes());
                bytes[9..13].copy_from_slice(&nodes.to_le_bytes());
                13
            }
            Self::Enter { object, formula } => {
                bytes[0] = 3;
                put_pair(&mut bytes, object, formula);
                9
            }
            Self::Finish { result, cache_slot } => {
                if cache_slot == Some(u32::MAX) {
                    return Err("reserved cache slot".into());
                }
                bytes[0] = 4;
                put_pair(&mut bytes, result, cache_slot.unwrap_or(u32::MAX));
                9
            }
            Self::Reuse(h) => {
                bytes[0] = 5;
                bytes[1..5].copy_from_slice(&h.slot.to_le_bytes());
                bytes[5..13].copy_from_slice(&h.generation.to_le_bytes());
                13
            }
        };
        out.write_all(&bytes[..n])
            .map_err(|e| format!("certificate write: {e}"))
    }

    // The caller consumes tag6 and its bounded terminal payload separately.
    pub(super) fn read(tag: u8, input: &mut impl Read) -> Result<Self, String> {
        Ok(match tag {
            0 => Self::Atom(word(input)?),
            1 => Self::Pair {
                left: index(input)?,
                right: index(input)?,
            },
            2 => Self::Reset {
                epoch: word(input)?,
                nodes: index(input)?,
            },
            3 => Self::Enter {
                object: index(input)?,
                formula: index(input)?,
            },
            4 => {
                let result = index(input)?;
                let slot = index(input)?;
                Self::Finish {
                    result,
                    cache_slot: (slot != u32::MAX).then_some(slot),
                }
            }
            5 => Self::Reuse(CacheHandle {
                slot: index(input)?,
                generation: word(input)?,
            }),
            _ => return Err("unknown certificate record".into()),
        })
    }
}

fn put_pair(bytes: &mut [u8], a: u32, b: u32) {
    bytes[1..5].copy_from_slice(&a.to_le_bytes());
    bytes[5..9].copy_from_slice(&b.to_le_bytes());
}
pub(super) fn read<const N: usize>(input: &mut impl Read) -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    input
        .read_exact(&mut bytes)
        .map_err(|e| format!("certificate read: {e}"))?;
    Ok(bytes)
}
pub(super) fn word(input: &mut impl Read) -> Result<u64, String> {
    Ok(u64::from_le_bytes(read(input)?))
}
pub(super) fn index(input: &mut impl Read) -> Result<u32, String> {
    Ok(u32::from_le_bytes(read(input)?))
}
