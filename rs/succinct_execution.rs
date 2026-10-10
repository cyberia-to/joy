//! Succinct public execution proofs: zheng's `ZHENGPF1` envelope, profile 1.
//!
//! The statement is the one public v3 proves (program, inputs, outputs,
//! cycles, budget); the witness is committed, not disclosed. The scheme and
//! its parameters are zheng's per-size-class choice
//! (`zheng::execution::succinct::prove_default`); the verifier admits any
//! scheme zheng names on the wire whose proven bound is ≥ 128 bits.
use crate::execution::{parse_program, render_program};
use crate::Warrior;
use std::{fs, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, Runner};
use zheng::envelope::{AnySuccinct, Envelope, SuccinctStatement, MAGIC};
use zheng::execution::ExecutionStatement;

/// Identifies the succinct execution profile.
pub const SUCCINCT_EXECUTION_FORMAT: &str = zheng::execution::succinct::FORMAT;
const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;

/// A succinct execution proof: exactly a zheng profile-1 envelope with an
/// execution statement.
#[derive(Clone, Debug)]
pub struct SuccinctExecutionArtifact {
    pub statement: ExecutionStatement,
    pub proof: AnySuccinct,
}

impl SuccinctExecutionArtifact {
    /// A `ZHENGPF1` file whose profile byte is 1.
    pub fn has_header(path: &Path) -> bool {
        let mut header = MAGIC.to_vec();
        header.extend_from_slice(&zheng::envelope::VERSION.to_le_bytes());
        header.push(zheng::envelope::Profile::Succinct as u8);
        crate::file_input::has_header(path, &header, MAX_ARTIFACT_BYTES)
    }
    fn envelope(&self) -> Envelope {
        Envelope::Succinct {
            statement: SuccinctStatement::Execution(self.statement.clone()),
            proof: self.proof.clone(),
        }
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let bytes = self.envelope().to_bytes();
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("succinct artifact size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("succinct artifact size limit".into());
        }
        match Envelope::from_bytes(bytes).map_err(|e| e.to_string())? {
            Envelope::Succinct {
                statement: SuccinctStatement::Execution(statement),
                proof,
            } => Ok(Self { statement, proof }),
            _ => Err("not a succinct execution envelope".into()),
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
    pub fn matches_program(&self, assembly: &str) -> Result<bool, String> {
        Ok(
            ExecutionStatement::encode_program(&parse_program(assembly)?)?
                == self.statement.program,
        )
    }
    pub fn assembly(&self) -> Result<String, String> {
        render_program(&self.statement.program)
    }
    /// Derive the relation from the statement and verify the proof.
    pub fn verify(&self) -> Result<(), String> {
        self.envelope().verify(None)
    }
    pub fn claim_matches(
        &self,
        claim: Option<&[u64]>,
        inputs: Option<&[u64]>,
        budget: u64,
    ) -> bool {
        claim.is_none_or(|c| c == self.statement.public_output)
            && inputs.is_none_or(|i| i == self.statement.public_input)
            && self.statement.budget <= budget
    }
}

/// What `joy prove --succinct` produces: one relation's succinct proof
/// (profile 1) when the program fits the relation compiler, otherwise a
/// machine proof (profile 4: the uniform step relation, accumulated).
#[derive(Clone, Debug)]
pub enum SuccinctArtifact {
    Relation(SuccinctExecutionArtifact),
    Machine(Box<crate::MachineExecutionArtifact>),
}

impl SuccinctArtifact {
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        match self {
            Self::Relation(a) => a.to_bytes(),
            Self::Machine(a) => a.to_bytes(),
        }
    }
}

impl Warrior {
    /// `joy prove --succinct`: one relation if the program fits the
    /// relation compiler, else the machine with accumulation.
    pub fn prove_succinct(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        budget: u64,
    ) -> Result<(SuccinctArtifact, ExecutionResult), String> {
        match self.prove_succinct_execution(bundle, input, budget) {
            Ok((a, r)) => Ok((SuccinctArtifact::Relation(a), r)),
            Err(relation) => self
                .prove_machine_execution(bundle, input, budget)
                .map(|(a, r)| (SuccinctArtifact::Machine(Box::new(a)), r))
                .map_err(|machine| format!("{relation}; machine: {machine}")),
        }
    }

    /// Produce a succinct public execution proof of one relation. Native
    /// execution is an independent prover check, as for public v3.
    pub fn prove_succinct_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        budget: u64,
    ) -> Result<(SuccinctExecutionArtifact, ExecutionResult), String> {
        if bundle.target_vm != "nox" || bundle.reads_state || !input.digests.is_empty() {
            return Err("succinct execution proofs require stateless nox inputs".into());
        }
        if !input.secret.is_empty() {
            return Err(
                "succinct execution proofs are not zero knowledge; secret inputs are refused"
                    .into(),
            );
        }
        let program = parse_program(&bundle.assembly)?;
        let (statement, proof) =
            zheng::execution::succinct::prove_default(&program, &input.public, budget)?;
        let native = Warrior::with_budget(budget).run(bundle, input)?;
        if native.output != statement.public_output || native.cycle_count != statement.cycles {
            return Err("execution relation disagrees with native nox".into());
        }
        Ok((SuccinctExecutionArtifact { statement, proof }, native))
    }
}
