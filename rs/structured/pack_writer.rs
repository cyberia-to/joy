//! Construct canonical trees from bounded flat host slices, never expand a DAG.
use super::reader::Result;
use nox::{Order, Reduction};
use std::time::Instant;

pub(super) struct Writer<'a, const N: usize> {
    pub ar: &'a mut Reduction<N>,
    pub deadline: Instant,
}

impl<const N: usize> Writer<'_, N> {
    fn check(&self) -> Result<()> {
        if Instant::now() >= self.deadline {
            Err("package construction deadline exceeded".into())
        } else {
            Ok(())
        }
    }

    pub fn atom(&mut self, value: u64) -> Result<Order> {
        self.check()?;
        if value >= 0xffff_ffff_0000_0001 {
            return Err("noncanonical package field".into());
        }
        self.ar
            .atom(nebu::Goldilocks::new(value))
            .ok_or_else(|| "package arena exhausted".into())
    }

    pub fn pair(&mut self, left: Order, right: Order) -> Result<Order> {
        self.check()?;
        self.ar
            .pair(left, right)
            .ok_or_else(|| "package arena exhausted".into())
    }

    pub fn record(&mut self, tag: u64, fields: &[Order]) -> Result<Order> {
        let mut body = self.atom(0)?;
        for &field in fields.iter().rev() {
            body = self.pair(field, body)?;
        }
        let tag = self.atom(tag)?;
        self.pair(tag, body)
    }

    // Height derives from a bounded flat slice. Recursion is at most32 and
    // never follows attacker-supplied arena links.
    fn tree(&mut self, values: &[Order], height: u32) -> Result<Order> {
        if values.is_empty() {
            let mut empty = self.atom(0)?;
            for _ in 0..height {
                empty = self.pair(empty, empty)?;
            }
            return Ok(empty);
        }
        if height == 0 {
            return Ok(values[0]);
        }
        let split = values.len().min(1usize << (height - 1));
        let left = self.tree(&values[..split], height - 1)?;
        let right = self.tree(&values[split..], height - 1)?;
        self.pair(left, right)
    }

    fn wrap(&mut self, tag: u64, length: u32, values: &[Order]) -> Result<Order> {
        let count = u32::try_from(values.len()).map_err(|_| "package collection length")?;
        let height = if count <= 1 {
            0
        } else {
            32 - (count - 1).leading_zeros()
        };
        let tree = self.tree(values, height)?;
        let length = self.atom(length as u64)?;
        let body = self.pair(length, tree)?;
        let tag = self.atom(tag)?;
        self.pair(tag, body)
    }

    pub fn seq(&mut self, values: &[Order]) -> Result<Order> {
        let length = u32::try_from(values.len()).map_err(|_| "package sequence length")?;
        self.wrap(0x53455131, length, values)
    }

    pub fn bytes(&mut self, value: &[u8]) -> Result<Order> {
        let length = u32::try_from(value.len()).map_err(|_| "package byte length")?;
        let mut words = Vec::new();
        for chunk in value.chunks(4) {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            words.push(self.atom(u32::from_le_bytes(word) as u64)?);
        }
        self.wrap(0x42595431, length, &words)
    }
}
