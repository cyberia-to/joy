use super::*;
use nebu::Goldilocks as F;
use nox::{CallProvider, LookProvider, Order, Outcome, Reduction, VecTrace};
struct Calls<'a> {
    state: &'a StateCertificate,
    secrets: &'a [u64],
    next: std::sync::atomic::AtomicUsize,
}
impl LookProvider for Calls<'_> {
    fn look(&self, _root: F, ns: F, key: F) -> Option<F> {
        self.state.cell(ns.as_u64(), key.as_u64()).map(F::new)
    }
}
impl<const N: usize> CallProvider<N> for Calls<'_> {
    fn provide(&self, arena: &mut Reduction<N>, _: F, _: Order) -> Option<Order> {
        let i = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        arena.atom(F::new(*self.secrets.get(i)?))
    }
}
impl Warrior {
    pub fn run_state_certificate(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        certificate: &StateCertificate,
        budget: u64,
    ) -> Result<ExecutionResult, String> {
        if bundle.target_vm != "nox"
            || !input.digests.is_empty()
            || input
                .public
                .iter()
                .chain(&input.secret)
                .any(|&v| v >= nebu::field::P)
        {
            return Err("invalid state execution inputs".into());
        }
        let root = certificate.root()?;
        certificate.verify(root)?;
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .name("joy-state-reduce".into())
                .stack_size(256 * 1024 * 1024)
                .spawn_scoped(scope, move || {
                    let mut arena = Reduction::<{ 1 << 18 }>::new();
                    let formula = crate::formula::parse(&mut arena, bundle.assembly.trim())?;
                    let mut object = crate::formula::build_subject(&mut arena, &input.public)?;
                    if bundle.reads_state {
                        let mut tail = arena
                            .atom(F::new(root[3]))
                            .ok_or("state root allocation failed")?;
                        for &limb in root[..3].iter().rev() {
                            let head = arena
                                .atom(F::new(limb))
                                .ok_or("state root allocation failed")?;
                            tail = arena
                                .pair(head, tail)
                                .ok_or("state root allocation failed")?;
                        }
                        object = arena
                            .pair(tail, object)
                            .ok_or("state subject allocation failed")?;
                    }
                    let mut trace = VecTrace::default();
                    let calls = Calls {
                        state: certificate,
                        secrets: &input.secret,
                        next: std::sync::atomic::AtomicUsize::new(0),
                    };
                    let result = match nox::reduce(
                        &mut arena, object, formula, budget, &calls, &mut trace,
                    ) {
                        Outcome::Ok(result, _) => result,
                        _ => return Err("native public state execution failed".into()),
                    };
                    for row in &trace.0 {
                        if row.r()[0] == 17
                            && [row.r()[4], row.r()[11], row.r()[12], row.r()[13]] != root
                        {
                            return Err("native lookup used another state root".into());
                        }
                    }
                    Ok(ExecutionResult {
                        output: crate::formula::leaves(&arena, result)?,
                        cycle_count: trace.0.len() as u64,
                    })
                })
                .map_err(|e| e.to_string())?
                .join()
                .map_err(|_| "state execution worker failed".to_string())?
        })
    }
}
