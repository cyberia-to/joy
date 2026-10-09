//! Machine execution proofs: zheng's `ZHENGPF1` envelope, profile 4.
//!
//! The statement is a nox run of any length — program, public inputs, the
//! output noun, cycles, budget — proven by zheng's uniform step relation,
//! its committed words accumulated and decided once (zheng
//! `specs/machine.md`, `specs/accumulation.md`). `joy prove --succinct`
//! produces one when the program exceeds the one-relation compiler.
use crate::execution::parse_machine_program as parse_program;
use crate::Warrior;
use std::{fs, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, Runner};
use zheng::envelope::{Envelope, Profile, MAGIC, VERSION};
use zheng::execution::ExecutionNoun;
use zheng::machine::{self, MachineProof, MachineStatement};

/// Identifies the machine profile.
pub const MACHINE_EXECUTION_FORMAT: &str = "zheng-nox-machine-v1";
const MAX_ARTIFACT_BYTES: usize = 256 * 1024 * 1024;

/// A machine execution proof: exactly a zheng profile-4 envelope.
#[derive(Clone, Debug)]
pub struct MachineExecutionArtifact {
    pub params: zheng::execution::succinct::WhirParams,
    pub statement: MachineStatement,
    pub proof: MachineProof,
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

impl MachineExecutionArtifact {
    /// A `ZHENGPF1` file whose profile byte is 4.
    pub fn has_header(path: &Path) -> bool {
        let mut header = MAGIC.to_vec();
        header.extend_from_slice(&VERSION.to_le_bytes());
        header.push(Profile::Machine as u8);
        crate::file_input::has_header(path, &header, MAX_ARTIFACT_BYTES)
    }
    fn envelope(&self) -> Envelope {
        Envelope::Machine {
            params: self.params,
            statement: self.statement.clone(),
            proof: Box::new(self.proof.clone()),
        }
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let bytes = self.envelope().to_bytes();
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("machine artifact size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err("machine artifact size limit".into());
        }
        match Envelope::from_bytes(bytes).map_err(|e| e.to_string())? {
            Envelope::Machine {
                params,
                statement,
                proof,
            } => Ok(Self {
                params,
                statement,
                proof: *proof,
            }),
            _ => Err("not a machine execution envelope".into()),
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
    /// Derive the relation from the statement and verify the proof.
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
    /// Prove a public execution with the machine (any length). Native
    /// execution is an independent prover check.
    pub fn prove_machine_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        budget: u64,
    ) -> Result<(MachineExecutionArtifact, ExecutionResult), String> {
        if bundle.target_vm != "nox" || bundle.reads_state || !input.digests.is_empty() {
            return Err("machine execution proofs require stateless nox inputs".into());
        }
        if !input.secret.is_empty() {
            return Err("machine execution proofs are not zero knowledge; secret inputs are refused".into());
        }
        let program = parse_program(&bundle.assembly)?;
        let params = zheng::execution::succinct::params_for(20);
        let (statement, proof) = machine::prove(&program, &input.public, budget, &params)?;
        let artifact = MachineExecutionArtifact {
            params,
            statement,
            proof,
        };
        let native = Warrior::with_budget(budget).run(bundle, input)?;
        if native.output != artifact.output()? || native.cycle_count != artifact.statement.cycles {
            return Err("machine relation disagrees with native nox".into());
        }
        Ok((artifact, native))
    }
}
