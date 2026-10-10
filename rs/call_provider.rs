//! Sequential native witnesses for call patterns.
use nebu::Goldilocks;
use nox::{CallProvider, LookProvider, Order, Reduction};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Serves secret inputs to nox call patterns (tag 16), in order.
///
/// Trident's `divine()` lowers to a call pattern; the prover-side
/// witness stream is the `--secret` input list. Each `provide()`
/// consumes the next value regardless of tag.
pub(super) struct SecretProvider {
    values: Vec<u64>,
    next: AtomicUsize,
}

impl SecretProvider {
    pub(super) fn new(values: Vec<u64>) -> Self {
        Self {
            values,
            next: AtomicUsize::new(0),
        }
    }
    pub(super) fn finish(&self) -> Result<(), String> {
        if self.next.load(Ordering::SeqCst) != self.values.len() {
            return Err("unused secret inputs".into());
        }
        Ok(())
    }
}

impl LookProvider for SecretProvider {
    fn look(
        &self,
        _commitment: Goldilocks,
        _namespace: Goldilocks,
        _key: Goldilocks,
    ) -> Option<Goldilocks> {
        None // Stateless execution has no state provider.
    }
}

impl<const N: usize> CallProvider<N> for SecretProvider {
    fn provide(
        &self,
        reduction: &mut Reduction<N>,
        _tag: Goldilocks,
        _object: Order,
    ) -> Option<Order> {
        let i = self.next.fetch_add(1, Ordering::SeqCst);
        let v = *self.values.get(i)?;
        reduction.atom(Goldilocks::new(v))
    }
}
