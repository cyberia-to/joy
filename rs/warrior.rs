//! warrior — Runner/Prover/Verifier/Deployer over nox reduce().
//!
//! Runner and re-execution verification. Proving and proof verification
//! live in `traits.rs` (public v3, native private, state v3). Deploy is an
//! honest dash (post-M4: particle + cyberlink emission). Never fake a proof.

use nox::{reduce_with_registry, ErrorKind, JetRegistry, Outcome, Reduction, VecTrace};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, Runner};

use crate::formula;

#[path = "call_provider.rs"]
mod call_provider;
#[path = "traits.rs"]
mod traits;
use call_provider::SecretProvider;

/// Default reduction budget: bounds trace rows one-to-one.
pub const DEFAULT_BUDGET: u64 = 1_000_000;

/// Arena capacity in data nodes (~26 MB of worker stack at 100 B/node).
const ARENA: usize = 1 << 18;

/// Worker thread stack: arena + parse recursion + reduce recursion.
const STACK_SIZE: usize = 256 * 1024 * 1024;

fn describe(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::TypeError => "type error (atom/pair mismatch)",
        ErrorKind::AxisError => "axis out of range",
        ErrorKind::InvZero => "inverse of zero",
        ErrorKind::Unavailable => "arena full or resource unavailable",
        ErrorKind::Malformed => "malformed formula",
        ErrorKind::CallRejected => "call witness rejected by check formula",
    }
}

/// The nox warrior. Executes ProgramBundles whose `target_vm` is "nox".
pub struct Warrior {
    budget: u64,
}

impl Warrior {
    pub fn new() -> Self {
        Self {
            budget: DEFAULT_BUDGET,
        }
    }

    pub fn with_budget(budget: u64) -> Self {
        Self { budget }
    }

    /// Execute the bundle's formula on a dedicated worker thread
    /// (the arena lives on the stack; the main thread is too small).
    fn execute(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
    ) -> Result<ExecutionResult, String> {
        if bundle.target_vm != "nox" {
            return Err(format!(
                "joy runs nox bundles; this bundle targets '{}'",
                bundle.target_vm
            ));
        }
        if bundle.reads_state {
            return Err(
                "bundle requires state; stateless execution cannot supply a state root".into(),
            );
        }
        if !input.digests.is_empty() {
            return Err("nox has no digest input stream".to_string());
        }

        if input
            .public
            .iter()
            .chain(&input.secret)
            .any(|&v| v >= nebu::field::P)
        {
            return Err("execution inputs must be canonical field elements".into());
        }
        let assembly = bundle.assembly.clone();
        let public = input.public.clone();
        let secret = input.secret.clone();
        let budget = self.budget;

        let handle = std::thread::Builder::new()
            .name("joy-reduce".to_string())
            .stack_size(STACK_SIZE)
            .spawn(move || -> Result<ExecutionResult, String> {
                let mut reduction = Reduction::<ARENA>::new();
                let root = formula::parse(&mut reduction, assembly.trim())?;
                let object = formula::build_subject(&mut reduction, &public)?;
                let provider = SecretProvider::new(secret);
                let mut tracer = VecTrace::default();
                // jets change speed, never results: zheng's hemera node
                // (one permutation instead of ~40k reductions, the same cost)
                let mut registry = JetRegistry::empty();
                zheng::execution::node::jet::register(&mut registry);
                match reduce_with_registry(
                    &mut reduction,
                    object,
                    root,
                    budget,
                    &provider,
                    &mut tracer,
                    &registry,
                ) {
                    Outcome::Ok(result, remaining) => {
                        provider.finish()?;
                        Ok(ExecutionResult {
                            output: formula::leaves(&reduction, result)?,
                            // the budget spent: a jet records one trace row
                            // for its whole cost
                            cycle_count: budget - remaining,
                        })
                    }
                    Outcome::Halt(remaining) => Err(format!(
                        "execution halted (budget remaining: {}) — out of budget, \
                         or a call pattern had no witness (secret inputs exhausted)",
                        remaining
                    )),
                    Outcome::Error(kind) => Err(format!("reduction error: {}", describe(kind))),
                }
            })
            .map_err(|e| format!("cannot spawn reduce thread: {}", e))?;

        handle
            .join()
            .map_err(|_| "reduce thread panicked".to_string())?
    }

    /// Verify a claimed output by re-execution (nox's unconditional mode).
    /// Public execution certificates are verified separately without native replay.
    pub fn verify_by_rerun(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        claim: &[u64],
    ) -> Result<bool, String> {
        let result = self.execute(bundle, input)?;
        Ok(result.output == claim)
    }
}

impl Default for Warrior {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for Warrior {
    fn run(&self, bundle: &ProgramBundle, input: &ProgramInput) -> Result<ExecutionResult, String> {
        self.execute(bundle, input)
    }
}
