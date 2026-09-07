//! warrior — Runner/Prover/Verifier/Deployer over nox reduce().
//!
//! Runner, re-execution verification, zheng proving and zheng proof
//! verification are real. Deploy is an honest dash (post-M4: particle +
//! cyberlink emission). Never fake a proof.

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
        self.execute_traced(bundle, input).map(|(r, _)| r)
    }

    /// Execute and keep the trace — the zheng witness.
    fn execute_traced(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
    ) -> Result<(ExecutionResult, VecTrace), String> {
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
            .spawn(move || -> Result<(ExecutionResult, VecTrace), String> {
                let mut reduction = Reduction::<ARENA>::new();
                let root = formula::parse(&mut reduction, assembly.trim())?;
                let object = formula::build_subject(&mut reduction, &public)?;
                let provider = SecretProvider::new(secret);
                let mut tracer = VecTrace::default();
                match reduce(&mut reduction, object, root, budget, &provider, &mut tracer) {
                    Outcome::Ok(result, _remaining) => Ok((
                        ExecutionResult {
                            output: formula::leaves(&reduction, result)?,
                            cycle_count: tracer.0.len() as u64,
                        },
                        tracer,
                    )),
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

impl Warrior {
    /// Execute the bundle and produce a zheng proof artifact.
    ///
    /// The statement binds: program_hash (hemera of the assembly),
    /// input_hash/output_hash (hemera of the first/last trace rows),
    /// focus_bound (the budget), bbg_root = zero sentinel (stateless).
    ///
    /// Honest gaps, refused rather than papered over: traces containing
    /// hash blocks (tag 15) need HashAux wiring; look rows (tag 17) need
    /// a bbg state and its root in the statement (M6 consumer side).
    pub fn prove_zheng(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
    ) -> Result<(crate::proof::ProofArtifact, ExecutionResult), String> {
        let (result, trace) = self.execute_traced(bundle, input)?;
        if trace.0.len() < 2 {
            return Err(format!(
                "trace has {} row(s): zheng folds row transitions and needs at least 2",
                trace.0.len()
            ));
        }
        if trace.0.iter().any(|r| r.r()[0] == 15) {
            return Err(
                "trace contains hash blocks (tag 15): HashAux wiring is not built yet"
                    .to_string(),
            );
        }
        if trace.0.iter().any(|r| r.r()[0] == 17) {
            return Err(
                "trace contains look rows (tag 17): proving state reads needs a bbg \
                 state and its root in the statement (soft3 M6)"
                    .to_string(),
            );
        }

        let first = &trace.0[0];
        let last = &trace.0[trace.0.len() - 1];
        let statement = zheng::Statement {
            program_hash: crate::proof::program_hash(&bundle.assembly),
            input_hash: zheng::row_hash(first),
            output_hash: zheng::row_hash(last),
            focus_bound: self.budget,
            bbg_root: [0u8; 32],
        };
        let params = zheng::ProofParams::default();
        let proof = zheng::commit(&trace, &[], &[], &[], &statement, &params)
            .map_err(|e| format!("zheng commit failed: {:?}", e))?;

        let artifact = crate::proof::ProofArtifact {
            format: crate::proof::PROOF_FORMAT.to_string(),
            statement,
            proof,
            meta: crate::proof::ArtifactMeta {
                program: bundle.name.clone(),
                output: result.output.clone(),
                cycle_count: result.cycle_count,
            },
        };
        Ok((artifact, result))
    }

    /// Verify a proof artifact against a bundle — no re-execution.
    ///
    /// Checks (1) the artifact's program_hash names this bundle's
    /// assembly, (2) the zheng proof verifies against the statement.
    pub fn verify_zheng(
        &self,
        bundle: &ProgramBundle,
        artifact: &crate::proof::ProofArtifact,
    ) -> Result<bool, String> {
        if artifact.statement.program_hash != crate::proof::program_hash(&bundle.assembly) {
            return Ok(false); // proof is for a different program
        }
        let params = zheng::ProofParams::default();
        Ok(zheng::verify(&artifact.proof, &artifact.statement, &params).is_ok())
    }
}

impl Prover for Warrior {
    fn prove(&self, bundle: &ProgramBundle, input: &ProgramInput) -> Result<ProofData, String> {
        let (artifact, result) = self.prove_zheng(bundle, input)?;
        let proof_bytes = serde_json::to_vec(&artifact)
            .map_err(|e| format!("cannot serialize proof artifact: {}", e))?;
        Ok(ProofData {
            claim: trident::field::proof::Claim {
                program_hash: artifact
                    .statement
                    .program_hash
                    .chunks_exact(8)
                    .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect(),
                public_input: input.public.clone(),
                public_output: result.output,
            },
            proof_bytes,
            format: crate::proof::PROOF_FORMAT.to_string(),
        })
    }
}

impl Verifier for Warrior {
    fn verify(&self, proof: &ProofData) -> Result<bool, String> {
        if proof.format != crate::proof::PROOF_FORMAT {
            return Err(format!(
                "unknown proof format '{}' (this joy verifies '{}')",
                proof.format,
                crate::proof::PROOF_FORMAT
            ));
        }
        let artifact: crate::proof::ProofArtifact = serde_json::from_slice(&proof.proof_bytes)
            .map_err(|e| format!("malformed proof bytes: {}", e))?;
        // No bundle in this trait call: verify the statement as carried.
        // Binding a proof to a specific bundle is verify_zheng's job.
        let params = zheng::ProofParams::default();
        Ok(zheng::verify(&artifact.proof, &artifact.statement, &params).is_ok())
    }
}

impl Deployer for Warrior {
    fn deploy(&self, _bundle: &ProgramBundle, _proof: Option<&ProofData>) -> Result<String, String> {
        Err("deploy lands after the zheng prover (M4): particle + cyberlink emission".to_string())
    }
}
