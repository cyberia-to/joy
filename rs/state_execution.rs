//! Public state execution with complete authenticated BBG dimension evidence.
use crate::{execution::parse_program, Warrior};
use bbg::{certificate::StateCertificate, BbgState};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, ProofData};
use zheng::execution::{state::StateStatement, DirectProof, ExecutionStatement};

pub const STATE_EXECUTION_FORMAT: &str = "joy-nox-public-state-execution-v1";
const MAGIC: &[u8; 8] = b"JOYST001";
const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateExecutionArtifact {
    pub format: String,
    pub program: String,
    pub assembly: String,
    pub source_hash: String,
    pub statement: StateStatement,
    pub certificate: StateCertificate,
    pub proof: DirectProof,
}
fn context(program: &str, source_hash: &str) -> [u8; 32] {
    let mut h = hemera::Hasher::new();
    h.update(STATE_EXECUTION_FORMAT.as_bytes());
    for s in [program, source_hash] {
        h.update(&(s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    }
    *h.finalize().as_bytes()
}
impl StateExecutionArtifact {
    pub fn matches_program(&self, bundle: &ProgramBundle) -> Result<bool, String> {
        Ok(bundle.target_vm == "nox"
            && bundle.reads_state == self.statement.root_in_subject
            && bundle.source_hash == self.source_hash
            && ExecutionStatement::encode_program(&parse_program(&bundle.assembly)?)?
                == self.statement.execution.program)
    }
    pub fn verify(&self) -> Result<(), String> {
        if self.format != STATE_EXECUTION_FORMAT
            || self.program.len() > 4096
            || self.source_hash.is_empty()
            || self.source_hash.len() > 128
            || self.statement.context != context(&self.program, &self.source_hash)
            || ExecutionStatement::encode_program(&parse_program(&self.assembly)?)?
                != self.statement.execution.program
        {
            return Err("invalid state execution identity".into());
        }
        self.certificate.verify(self.statement.state_root)?;
        self.statement
            .verify(&self.proof, &mut |ns, key| self.certificate.cell(ns, key))
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(postcard::to_allocvec(self).map_err(|e| e.to_string())?);
        if bytes.len() > MAX_BYTES {
            return Err("state execution artifact size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("state execution artifact size limit".into());
        }
        let payload = bytes
            .strip_prefix(MAGIC)
            .ok_or("not a state execution artifact")?;
        let (artifact, rest): (Self, &[u8]) =
            postcard::take_from_bytes(payload).map_err(|e| e.to_string())?;
        if !rest.is_empty() || artifact.format != STATE_EXECUTION_FORMAT {
            return Err("invalid state execution wire format".into());
        }
        Ok(artifact)
    }
    pub fn has_header(path: &Path) -> bool {
        crate::file_input::has_header(path, MAGIC, MAX_BYTES)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::from_bytes(&read_bounded(path)?)
    }
    pub fn save(&self, path: &Path) -> Result<usize, String> {
        let bytes = self.to_bytes()?;
        fs::write(path, &bytes).map_err(|e| e.to_string())?;
        Ok(bytes.len())
    }
    pub fn proof_data(&self) -> Result<ProofData, String> {
        Ok(ProofData {
            format: STATE_EXECUTION_FORMAT.into(),
            proof_bytes: self.to_bytes()?,
            claim: trident::field::proof::Claim {
                program_hash: crate::program_hash(&self.assembly)
                    .chunks_exact(8)
                    .map(|b| {
                        let mut a = [0; 8];
                        a.copy_from_slice(b);
                        u64::from_le_bytes(a)
                    })
                    .collect(),
                public_input: self.statement.execution.public_input.clone(),
                public_output: self.statement.execution.public_output.clone(),
            },
        })
    }
}
fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    crate::file_input::read(path, MAX_BYTES)
}
/// The CLI state file is a public BBG certificate, not an unverified database dump.
pub fn load_certificate(path: &Path) -> Result<StateCertificate, String> {
    let certificate: StateCertificate =
        serde_json::from_slice(&read_bounded(path)?).map_err(|e| e.to_string())?;
    certificate.verify(certificate.root()?)?;
    Ok(certificate)
}
impl Warrior {
    pub fn prove_state_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        state: &BbgState,
        budget: u64,
    ) -> Result<(StateExecutionArtifact, ExecutionResult), String> {
        let certificate = StateCertificate::from_state(state, &(0..10).collect::<Vec<_>>())?;
        self.prove_state_certificate(bundle, input, &certificate, budget)
    }
    pub fn prove_state_certificate(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        certificate: &StateCertificate,
        budget: u64,
    ) -> Result<(StateExecutionArtifact, ExecutionResult), String> {
        check_input(bundle, input)?;
        let root = certificate.root()?;
        certificate.verify(root)?;
        let program = parse_program(&bundle.assembly)?;
        let (statement, proof) = zheng::execution::state::prove_state_execution(
            &program,
            &input.public,
            budget,
            root,
            bundle.reads_state,
            context(&bundle.name, &bundle.source_hash),
            &mut |ns, key| certificate.cell(ns, key),
        )?;
        let result = self.run_state_certificate(bundle, input, certificate, budget)?;
        if result.output != statement.execution.public_output
            || result.cycle_count != statement.execution.cycles
        {
            return Err("state execution relation disagrees with native nox".into());
        }
        let artifact = StateExecutionArtifact {
            format: STATE_EXECUTION_FORMAT.into(),
            program: bundle.name.clone(),
            assembly: bundle.assembly.clone(),
            source_hash: bundle.source_hash.clone(),
            statement,
            certificate: certificate.clone(),
            proof,
        };
        artifact.verify()?;
        Ok((artifact, result))
    }
}
fn check_input(bundle: &ProgramBundle, input: &ProgramInput) -> Result<(), String> {
    if bundle.target_vm != "nox" || !input.secret.is_empty() || !input.digests.is_empty() {
        return Err("public state execution requires nox with no secret/digest input".into());
    }
    if input.public.iter().any(|&v| v >= nebu::field::P) {
        return Err("noncanonical public input".into());
    }
    Ok(())
}
pub fn verify_proof_data(data: &ProofData) -> Result<bool, String> {
    if data.format != STATE_EXECUTION_FORMAT {
        return Err("unsupported state proof format".into());
    }
    let artifact = StateExecutionArtifact::from_bytes(&data.proof_bytes)?;
    Ok(artifact.proof_data()?.claim == data.claim && artifact.verify().is_ok())
}
#[path = "state_native.rs"]
mod native;
