//! Bounded schema traversal. No source parsing and no arena allocations.
use nox::{Digest, Order, Reduction};
use std::{sync::OnceLock, time::Instant};

pub(super) type Result<T> = std::result::Result<T, String>;

pub(super) struct Reader<'a, const N: usize> {
    pub ar: &'a Reduction<N>,
    pub remaining: u32,
    pub sequence_limit: u32,
    pub deadline: Instant,
}

fn empty_particles() -> &'static [Digest; 33] {
    static EMPTY: OnceLock<[Digest; 33]> = OnceLock::new();
    EMPTY.get_or_init(|| {
        let zero = nox::data::hash_atom(nebu::Goldilocks::new(0));
        let mut result = [zero; 33];
        for i in 1..33 {
            result[i] = nox::data::hash_pair(&result[i - 1], &result[i - 1]);
        }
        result
    })
}

impl<const N: usize> Reader<'_, N> {
    pub fn charge(&mut self, visits: u32) -> Result<()> {
        if Instant::now() >= self.deadline {
            return Err("execution deadline exceeded during schema admission".into());
        }
        self.remaining = self
            .remaining
            .checked_sub(visits)
            .ok_or("validation visits exhausted")?;
        Ok(())
    }

    pub fn field(&mut self, node: Order) -> Result<u64> {
        self.charge(1)?;
        self.ar
            .atom_value(node)
            .map(|v| v.as_u64())
            .ok_or_else(|| "expected Field atom".into())
    }

    pub fn word(&mut self, node: Order) -> Result<u32> {
        u32::try_from(self.field(node)?).map_err(|_| "expected U32 atom".into())
    }

    pub fn digest(&mut self, node: Order) -> Result<Digest> {
        self.charge(10)?; // Three pairs (head/tail) and four scalar limbs.
        self.ar
            .read_hash_data(node)
            .ok_or_else(|| "digest shape".into())
    }

    fn pair(&mut self, node: Order) -> Result<(Order, Order)> {
        self.charge(2)?;
        Ok((
            self.ar.head(node).ok_or("expected pair")?,
            self.ar.tail(node).ok_or("expected pair")?,
        ))
    }

    pub fn record<const F: usize>(&mut self, node: Order, tag: u64) -> Result<[Order; F]> {
        let (actual, mut body) = self.pair(node)?;
        if self.field(actual)? != tag {
            return Err(format!("record tag: expected {tag:#x}"));
        }
        let mut fields = [0; F];
        for field in &mut fields {
            (*field, body) = self.pair(body)?;
        }
        if self.field(body)? != 0 {
            return Err("record terminator".into());
        }
        Ok(fields)
    }

    fn wrapper(&mut self, node: Order, tag: u64, limit: u32) -> Result<(u32, Order)> {
        let (actual, body) = self.pair(node)?;
        if self.field(actual)? != tag {
            return Err("collection tag".into());
        }
        let (length, root) = self.pair(body)?;
        let length = self.word(length)?;
        if length > limit {
            return Err("collection length limit".into());
        }
        Ok((length, root))
    }

    // Traversal stack is bounded by the U32 tree height, including shared DAGs.
    fn tree(
        &mut self,
        root: Order,
        length: u32,
        mut leaf: impl FnMut(Order) -> Result<()>,
    ) -> Result<()> {
        let height = if length <= 1 {
            0
        } else {
            32 - (length - 1).leading_zeros()
        };
        let mut todo = vec![(root, length, height)];
        while let Some((node, occupied, h)) = todo.pop() {
            self.charge(1)?;
            if occupied == 0 {
                if self.ar.digest(node) != Some(&empty_particles()[h as usize]) {
                    return Err("collection padding".into());
                }
            } else if h == 0 {
                leaf(node)?;
            } else {
                let left = self.ar.head(node).ok_or("collection branch")?;
                let right = self.ar.tail(node).ok_or("collection branch")?;
                let half = 1u32 << (h - 1);
                todo.push((right, occupied.saturating_sub(half), h - 1));
                todo.push((left, occupied.min(half), h - 1));
            }
        }
        Ok(())
    }

    pub fn list(&mut self, node: Order, limit: u32) -> Result<Vec<Order>> {
        let (length, root) = self.wrapper(node, 0x53455131, limit.min(self.sequence_limit))?;
        // Every occupied leaf needs a visit, even when it shares an arena node.
        if length > self.remaining {
            return Err("validation visits exhausted".into());
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(length as usize)
            .map_err(|_| "sequence allocation")?;
        self.tree(root, length, |n| {
            values.push(n);
            Ok(())
        })?;
        Ok(values)
    }

    pub fn bytes(&mut self, node: Order, limit: u32) -> Result<Vec<u8>> {
        let (length, root) = self.wrapper(node, 0x42595431, limit)?;
        let words = length / 4 + u32::from(length % 4 != 0);
        if words > self.remaining {
            return Err("validation visits exhausted".into());
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length as usize)
            .map_err(|_| "byte allocation")?;
        let ar = self.ar;
        self.tree(root, words, |n| {
            let word = ar.atom_value(n).ok_or("byte word shape")?.as_u64();
            let word = u32::try_from(word).map_err(|_| "byte word range")?;
            let take = (length as usize - bytes.len()).min(4);
            if take < 4 && word >> (take * 8) != 0 {
                return Err("unused high byte padding".into());
            }
            bytes.extend_from_slice(&word.to_le_bytes()[..take]);
            Ok(())
        })?;
        Ok(bytes)
    }

    pub fn string(&mut self, node: Order, limit: u32) -> Result<String> {
        String::from_utf8(self.bytes(node, limit)?).map_err(|_| "invalid UTF-8 string".into())
    }
}

pub(super) fn identifier(value: &str) -> bool {
    value.len() <= 255
        && value
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

pub(super) fn module_path(value: &str) -> bool {
    value.len() <= 255 && value.split('.').all(identifier)
}

pub(super) fn label(value: &str) -> bool {
    !value.is_empty() && value.len() <= 255 && value.bytes().all(|c| (32..=126).contains(&c))
}
