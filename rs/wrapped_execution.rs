//! Wrapped execution proofs: zheng's `ZHENGPF1` envelope, profile 6.
//!
//! The statement is a nox run of any length, proven by IVC (profile 5)
//! whose final verifier is then proved by zheng's wrap levels: the final
//! proof is the recursive proof's header, the deferred nox-public claim and
//! one final-mode wrap proof — ≤ 64 KB, constant in the run's length
//! (zheng `specs/recursion.md` § wrap). Proving runs the IVC and the wrap
//! chain (minutes, tens of GB: `crate::memory`); verification needs the IVC
//! key and the final wrap key (`crate::key_cache`).
use crate::execution::parse_machine_program as parse_program;
use crate::Warrior;
use std::{fs, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, Runner};
use zheng::envelope::{wrapped, Envelope, Profile, MAGIC, VERSION};
use zheng::execution::ExecutionNoun;
use zheng::machine::{self, MachineStatement};
use zheng::recursion::wrap::FinalProof;

/// Identifies the wrapped profile.
pub const WRAPPED_EXECUTION_FORMAT: &str = "zheng-nox-wrapped-v1";
const MAX_ARTIFACT_BYTES: usize = 256 * 1024;

/// A wrapped execution proof: exactly a zheng profile-6 envelope.
#[derive(Clone, Debug)]
pub struct WrappedExecutionArtifact {
    pub statement: MachineStatement,
    pub proof: FinalProof,
}

fn leaves(noun: &ExecutionNoun, out: &mut Vec<u64>) {
    match noun {
        ExecutionNoun::Atom(v) => out.push(*v),
        ExecutionNoun::Pair(a, b) => {
            leaves(a, out);
            leaves(b, out);
        }
    }
}

impl WrappedExecutionArtifact {
    /// A `ZHENGPF1` file whose profile byte is 6.
    pub fn has_header(path: &Path) -> bool {
        let mut header = MAGIC.to_vec();
        header.extend_from_slice(&VERSION.to_le_bytes());
        header.push(Profile::Wrapped as u8);
        crate::file_input::has_header(path, &header, MAX_ARTIFACT_BYTES)
    }
    fn envelope(&self) -> Envelope {
        Envelope::Wrapped {
            statement: self.statement.clone(),
            proof: Box::new(self.proof.clone()),
        }
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let bytes = self.envelope().to_bytes();
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("wrapped artifact size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("wrapped artifact size limit".into());
        }
        match Envelope::from_bytes(bytes).map_err(|e| e.to_string())? {
            Envelope::Wrapped { statement, proof } => Ok(Self {
                statement,
                proof: *proof,
            }),
            _ => Err("not a wrapped execution envelope".into()),
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::from_bytes(&crate::file_input::read(path, MAX_ARTIFACT_BYTES)?)
    }
    pub fn save(&self, path: &Path) -> Result<usize, String> {
        let bytes = self.to_bytes()?;
        fs::write(path, &bytes).map_err(|e| e.to_string())?;
        Ok(bytes.len())
    }
    /// The output noun's atoms, left to right (joy's output convention).
    pub fn output(&self) -> Result<Vec<u64>, String> {
        let mut out = Vec::new();
        leaves(&machine::statement::parse(&self.statement.output)?, &mut out);
        Ok(out)
    }
    pub fn matches_program(&self, assembly: &str) -> Result<bool, String> {
        Ok(machine::statement::tokens(&parse_program(assembly)?) == self.statement.program)
    }
    /// Verify under the process's keys (installed from the cache, or
    /// derived over zheng's pinned roots).
    pub fn verify(&self) -> Result<(), String> {
        self.envelope().verify(None)
    }
    pub fn claim_matches(&self, claim: Option<&[u64]>, inputs: Option<&[u64]>, budget: u64) -> bool {
        claim.is_none_or(|c| self.output().is_ok_and(|o| o == c))
            && inputs.is_none_or(|i| i == self.statement.input)
            && self.statement.budget <= budget
    }
}

impl Warrior {
    /// Prove a public execution recursively, then wrap it (any length,
    /// ≤ 64 KB). Native execution is an independent prover check.
    pub fn prove_wrapped_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        budget: u64,
    ) -> Result<(WrappedExecutionArtifact, ExecutionResult), String> {
        if bundle.target_vm != "nox" || bundle.reads_state || !input.digests.is_empty() {
            return Err("wrapped execution proofs require stateless nox inputs".into());
        }
        if !input.secret.is_empty() {
            return Err("wrapped execution proofs are not zero knowledge; secret inputs are refused".into());
        }
        let program = parse_program(&bundle.assembly)?;
        let artifact = match wrapped::prove(&program, &input.public, budget)? {
            Envelope::Wrapped { statement, proof } => WrappedExecutionArtifact {
                statement,
                proof: *proof,
            },
            _ => return Err("wrapped prover returned another profile".into()),
        };
        let native = Warrior::with_budget(budget).run(bundle, input)?;
        if native.output != artifact.output()? || native.cycle_count != artifact.statement.cycles {
            return Err("wrapped relation disagrees with native nox".into());
        }
        Ok((artifact, native))
    }
}
