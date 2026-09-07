//! warrior — Runner/Prover/Verifier/Deployer over nox reduce().
//!
//! Runner and re-execution verification are real. Prover, proof
//! verification, and deploy are honest dashes until the zheng prover
//! lands (M4 of the soft3 release). Never fake a proof.

use std::sync::atomic::{AtomicUsize, Ordering};

use nebu::Goldilocks;
use nox::{reduce, ErrorKind, Order, Outcome, Reduction, VecTrace};
use nox::{CallProvider, LookProvider};
use trident::runtime::{
    Deployer, ExecutionResult, ProgramBundle, ProgramInput, ProofData, Prover, Runner, Verifier,
};

use crate::formula;

/// Default reduction budget: bounds trace rows one-to-one.
pub const DEFAULT_BUDGET: u64 = 1_000_000;

/// Arena capacity in data nodes (~26 MB of worker stack at 100 B/node).
const ARENA: usize = 1 << 18;

/// Worker thread stack: arena + parse recursion + reduce recursion.
const STACK_SIZE: usize = 256 * 1024 * 1024;

/// Serves secret inputs to nox call patterns (tag 16), in order.
///
/// Trident's `divine()` lowers to a call pattern; the prover-side
/// witness stream is the `--secret` input list. Each `provide()`
/// consumes the next value regardless of tag.
struct SecretProvider {
    values: Vec<u64>,
    next: AtomicUsize,
}

impl SecretProvider {
    fn new(values: Vec<u64>) -> Self {
        Self {
            values,
            next: AtomicUsize::new(0),
        }
    }
}

impl LookProvider for SecretProvider {
    fn look(
        &self,
        _commitment: Goldilocks,
        _namespace: Goldilocks,
        _key: Goldilocks,
    ) -> Option<Goldilocks> {
        None // bbg look arrives in M6
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
                "joy runs nox bundles; this bundle targets '{}' (use trisha for triton)",
                bundle.target_vm
            ));
        }
        if !input.digests.is_empty() {
            return Err(
                "nox has no digest input stream (merkle_step is a Triton concept)".to_string(),
            );
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
                match reduce(&mut reduction, object, root, budget, &provider, &mut tracer) {
                    Outcome::Ok(result, _remaining) => Ok(ExecutionResult {
                        output: formula::leaves(&reduction, result)?,
                        cycle_count: tracer.0.len() as u64,
                    }),
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
    /// zheng proof verification replaces this in M4.
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

impl Prover for Warrior {
    fn prove(&self, _bundle: &ProgramBundle, _input: &ProgramInput) -> Result<ProofData, String> {
        Err("zheng prover lands in M4 of the soft3 release".to_string())
    }
}

impl Verifier for Warrior {
    fn verify(&self, _proof: &ProofData) -> Result<bool, String> {
        Err("zheng proof verification lands in M4 of the soft3 release; \
             until then verify by re-execution: joy verify <bundle.json> --claim <values>"
            .to_string())
    }
}

impl Deployer for Warrior {
    fn deploy(&self, _bundle: &ProgramBundle, _proof: Option<&ProofData>) -> Result<String, String> {
        Err("deploy lands after the zheng prover (M4): particle + cyberlink emission".to_string())
    }
}
