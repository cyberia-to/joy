//! warrior — Runner/Prover/Verifier/Deployer over nox reduce().
//!
//! Runner, re-execution verification, zheng proving and zheng proof
//! verification are real. Deploy is an honest dash (post-M4: particle +
//! cyberlink emission). Never fake a proof.

use std::sync::atomic::{AtomicUsize, Ordering};

use bbg::query::ProofLookProvider;
use bbg::BbgState;
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

/// CallProvider over a live BBG state: looks answer (and record openings)
/// via [`ProofLookProvider`]; secrets serve call patterns as usual.
struct StateCalls<'a> {
    looks: ProofLookProvider<'a>,
    secrets: SecretProvider,
}

impl<'a> LookProvider for StateCalls<'a> {
    fn look(
        &self,
        commitment: Goldilocks,
        namespace: Goldilocks,
        key: Goldilocks,
    ) -> Option<Goldilocks> {
        self.looks.look(commitment, namespace, key)
    }
}

impl<'a, const N: usize> CallProvider<N> for StateCalls<'a> {
    fn provide(
        &self,
        reduction: &mut Reduction<N>,
        tag: Goldilocks,
        object: Order,
    ) -> Option<Order> {
        CallProvider::<N>::provide(&self.secrets, reduction, tag, object)
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

/// Build one [`zheng::HashAux`] per tag-15 block: the sponge rate is the
/// structural digest of the hashed input (r15 of the block's first row),
/// zero-padded to 8 — exactly what zheng's hash bindings replay against.
/// Must run while the arena is alive; the digest is cached by the hash
/// pattern itself.
fn hash_aux_from_trace<const N: usize>(
    reduction: &Reduction<N>,
    tracer: &VecTrace,
) -> Result<Vec<zheng::HashAux>, String> {
    let mut aux = Vec::new();
    let mut i = 0;
    while i < tracer.0.len() {
        if tracer.0[i].r()[0] != 15 {
            i += 1;
            continue;
        }
        let start = i;
        while i < tracer.0.len() && tracer.0[i].r()[0] == 15 {
            i += 1;
        }
        if i - start < 2 {
            continue; // stray row: no block, no aux (mirrors zheng's scan)
        }
        let input = tracer.0[start].r()[15] as Order;
        let digest = reduction
            .digest(input)
            .copied()
            .ok_or_else(|| "hash block input has no cached digest".to_string())?;
        let mut rate = [Goldilocks::ZERO; 8];
        rate[..4].copy_from_slice(&digest);
        aux.push(zheng::HashAux { rate });
    }
    Ok(aux)
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
        self.execute_traced(bundle, input).map(|(r, _, _)| r)
    }

    /// Execute and keep the trace — the zheng witness — plus the HashAux
    /// for every tag-15 block (the rate = structural digest of the hashed
    /// input, recoverable only while the arena is alive).
    fn execute_traced(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
    ) -> Result<(ExecutionResult, VecTrace, Vec<zheng::HashAux>), String> {
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
            .spawn(move || -> Result<(ExecutionResult, VecTrace, Vec<zheng::HashAux>), String> {
                let mut reduction = Reduction::<ARENA>::new();
                let root = formula::parse(&mut reduction, assembly.trim())?;
                let object = formula::build_subject(&mut reduction, &public)?;
                let provider = SecretProvider::new(secret);
                let mut tracer = VecTrace::default();
                match reduce(&mut reduction, object, root, budget, &provider, &mut tracer) {
                    Outcome::Ok(result, _remaining) => {
                        let aux = hash_aux_from_trace(&reduction, &tracer)?;
                        Ok((
                            ExecutionResult {
                                output: formula::leaves(&reduction, result)?,
                                cycle_count: tracer.0.len() as u64,
                            },
                            tracer,
                            aux,
                        ))
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
    /// Hash blocks (tag 15) prove via HashAux built from the arena's
    /// cached input digests. Honest gap, refused rather than papered
    /// over: look rows (tag 17) need a bbg state and its root in the
    /// statement — use [`Warrior::prove_zheng_with_state`].
    pub fn prove_zheng(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
    ) -> Result<(crate::proof::ProofArtifact, ExecutionResult), String> {
        let (result, trace, hash_aux) = self.execute_traced(bundle, input)?;
        if trace.0.iter().any(|r| r.r()[0] == 17) {
            return Err(
                "trace contains look rows (tag 17): proving state reads needs a bbg \
                 state — use prove_zheng_with_state"
                    .to_string(),
            );
        }
        self.finish_proof(bundle, result, trace, hash_aux, Vec::new(), [0u8; 32])
    }

    /// Execute against a live BBG state and produce a zheng proof artifact.
    ///
    /// Look rows (tag 17) answer from `state` and record Brakedown openings;
    /// the statement carries `state.root()` as the PUBLIC root — the full
    /// chain from the committed state to the verified proof.
    pub fn prove_zheng_with_state(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        state: &BbgState,
    ) -> Result<(crate::proof::ProofArtifact, ExecutionResult), String> {
        let (result, trace, hash_aux, look_openings) =
            self.execute_traced_with_state(bundle, input, state)?;
        self.finish_proof(bundle, result, trace, hash_aux, look_openings, state.root())
    }

    /// Build statement + zheng proof + artifact from an executed trace.
    fn finish_proof(
        &self,
        bundle: &ProgramBundle,
        result: ExecutionResult,
        trace: VecTrace,
        hash_aux: Vec<zheng::HashAux>,
        look_openings: Vec<zheng::LookOpening>,
        bbg_root: [u8; 32],
    ) -> Result<(crate::proof::ProofArtifact, ExecutionResult), String> {
        if trace.0.len() < 2 {
            return Err(format!(
                "trace has {} row(s): zheng folds row transitions and needs at least 2",
                trace.0.len()
            ));
        }
        let first = &trace.0[0];
        let last = &trace.0[trace.0.len() - 1];
        let statement = zheng::Statement {
            program_hash: crate::proof::program_hash(&bundle.assembly),
            input_hash: zheng::row_hash(first),
            output_hash: zheng::row_hash(last),
            focus_bound: self.budget,
            bbg_root,
        };
        let params = zheng::ProofParams::default();
        let proof = zheng::commit(&trace, &hash_aux, &[], &look_openings, &statement, &params)
            .map_err(|e| format!("zheng commit failed: {:?}", e))?;

        let artifact = crate::proof::ProofArtifact {
            format: crate::proof::PROOF_FORMAT.to_string(),
            statement,
            proof,
            meta: crate::proof::ArtifactMeta {
                program: bundle.name.clone(),
                output: result.output.clone(),
                cycle_count: result.cycle_count,
                assembly: None,
                assembly_deflate: Some(miniz_oxide::deflate::compress_to_vec(
                    bundle.assembly.as_bytes(),
                    9,
                )),
            },
        };
        Ok((artifact, result))
    }

    /// Execute with looks answered from a BBG state (scoped worker thread —
    /// the state reference cannot cross a 'static spawn).
    #[allow(clippy::type_complexity)]
    fn execute_traced_with_state(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        state: &BbgState,
    ) -> Result<
        (
            ExecutionResult,
            VecTrace,
            Vec<zheng::HashAux>,
            Vec<zheng::LookOpening>,
        ),
        String,
    > {
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

        std::thread::scope(|scope| {
            let handle = std::thread::Builder::new()
                .name("joy-reduce".to_string())
                .stack_size(STACK_SIZE)
                .spawn_scoped(scope, move || {
                    let mut reduction = Reduction::<ARENA>::new();
                    let root = formula::parse(&mut reduction, assembly.trim())?;
                    let object = formula::build_subject(&mut reduction, &public)?;
                    let provider = StateCalls {
                        looks: ProofLookProvider::new(state),
                        secrets: SecretProvider::new(secret),
                    };
                    let mut tracer = VecTrace::default();
                    match reduce(&mut reduction, object, root, budget, &provider, &mut tracer) {
                        Outcome::Ok(result, _remaining) => {
                            let aux = hash_aux_from_trace(&reduction, &tracer)?;
                            let openings = provider.looks.take_look_openings();
                            Ok((
                                ExecutionResult {
                                    output: formula::leaves(&reduction, result)?,
                                    cycle_count: tracer.0.len() as u64,
                                },
                                tracer,
                                aux,
                                openings,
                            ))
                        }
                        Outcome::Halt(remaining) => Err(format!(
                            "execution halted (budget remaining: {}) — out of budget, \
                             or a call pattern had no witness (secret inputs exhausted)",
                            remaining
                        )),
                        Outcome::Error(kind) => {
                            Err(format!("reduction error: {}", describe(kind)))
                        }
                    }
                })
                .map_err(|e| format!("cannot spawn reduce thread: {}", e))?;

            handle
                .join()
                .map_err(|_| "reduce thread panicked".to_string())?
        })
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

    /// Verify a self-contained artifact — no bundle, no re-execution.
    ///
    /// The assembly carried in `meta` is recomputed into program_hash and
    /// checked against the statement first, so a tampered assembly is
    /// rejected exactly like a tampered proof. This is what
    /// `trident verify <artifact>` reaches through the warrior boundary.
    pub fn verify_artifact(
        &self,
        artifact: &crate::proof::ProofArtifact,
    ) -> Result<bool, String> {
        let assembly = artifact.meta.assembly_text()?.ok_or_else(|| {
            "artifact carries no assembly (written before 0.2.0) — verify it \
             against its bundle: joy verify <bundle> --proof <artifact>"
                .to_string()
        })?;
        if artifact.statement.program_hash != crate::proof::program_hash(&assembly) {
            return Ok(false); // assembly in meta does not match the proven program
        }
        let params = zheng::ProofParams::default();
        Ok(zheng::verify(&artifact.proof, &artifact.statement, &params).is_ok())
    }
}

impl Prover for Warrior {
    fn prove(&self, bundle: &ProgramBundle, input: &ProgramInput) -> Result<ProofData, String> {
        let (artifact, result) = self.prove_zheng(bundle, input)?;
        let proof_bytes = artifact.to_bytes()?;
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
        let artifact = crate::proof::ProofArtifact::from_bytes(&proof.proof_bytes)?;
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
