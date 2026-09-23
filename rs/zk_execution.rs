//! Private nox execution: Zheng derives the relation; Trisha proves its exact
//! checker with Triton's randomized ZK STARK. This format never carries witness.
use crate::{execution::parse_program, Warrior};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, ProofData, Runner};
use trisha_rs::ccs::Checker;
use zheng::execution::{private::PrivateStatement, ExecutionStatement};

pub const ZK_EXECUTION_FORMAT: &str = "joy-nox-ccs-triton7-zk-v3";
const MAGIC: &[u8; 8] = b"JOYZK003";
const MAX_BYTES: usize = 64 * 1024 * 1024;

fn require_backend() -> Result<(), String> {
    if trisha_rs::ccs::FORMAT != "zheng-ccs-triton7-zk-v2" {
        return Err(
            "private execution requires the Triton7 CCS backend; rebuild the coordinated release"
                .into(),
        );
    }
    Ok(())
}

/// Public statement and opaque proof only. `source_hash` is compilation identity
/// bound to the proof; applications can compare it to their expected build.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkExecutionArtifact {
    pub format: String,
    pub program: String,
    pub assembly: String,
    pub source_hash: String,
    pub statement: PrivateStatement,
    pub proof: Vec<u8>,
    pub state: Option<bbg::certificate::StateCertificate>,
    pub root_in_subject: bool,
}

impl ZkExecutionArtifact {
    fn binding(&self) -> [u8; 32] {
        let mut hasher = hemera::Hasher::new();
        hasher.update(b"joy-private-execution-statement-v3");
        for bytes in [
            self.statement.transcript_bytes().as_slice(),
            self.source_hash.as_bytes(),
            self.program.as_bytes(),
        ] {
            hasher.update(&(bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        }
        hasher.update(&[
            u8::from(self.root_in_subject),
            u8::from(self.state.is_some()),
        ]);
        if let Some(state) = &self.state {
            // These authenticated leaves fix every included public table.
            hasher.update(&state.version.to_le_bytes());
            hasher.update(&state.lens_version.to_le_bytes());
            for v in state.leaves.iter().flatten() {
                hasher.update(&v.to_le_bytes());
            }
        }
        *hasher.finalize().as_bytes()
    }
    fn validate(&self) -> Result<(), String> {
        require_backend()?;
        if self.format != ZK_EXECUTION_FORMAT
            || self.program.len() > 4096
            || self.source_hash.is_empty()
            || self.source_hash.len() > 128
            || self.proof.len() > MAX_BYTES
        {
            return Err("invalid private execution metadata".into());
        }
        if self.state.is_none() && self.root_in_subject {
            return Err("state ABI requires a certificate".into());
        }
        let program = parse_program(&self.assembly)?;
        if ExecutionStatement::encode_program(&program)? != self.statement.execution.program {
            return Err("assembly differs from private execution statement".into());
        }
        Ok(())
    }
    pub fn verify(&self) -> Result<(), String> {
        self.validate()?;
        let prepared = if let Some(state) = &self.state {
            let tables = state_tables(state)?;
            zheng::execution::private_state::PrivateStateStatement {
                execution: self.statement.execution.clone(),
                root: tables.root.map(|v| v.as_u64()),
                root_in_subject: self.root_in_subject,
            }
            .prepare(&tables)?
        } else {
            self.statement.prepare()?
        };
        let checker = Checker::new(
            &prepared.relation.instance,
            &prepared.public_coordinates,
            &self.binding(),
        )?;
        let proof = trisha_rs::ccs::decode_proof(&self.proof)?;
        if !checker.verify(&proof)? {
            return Err("private execution proof rejected".into());
        }
        Ok(())
    }
    pub fn matches_program(&self, assembly: &str) -> Result<bool, String> {
        Ok(
            ExecutionStatement::encode_program(&parse_program(assembly)?)?
                == self.statement.execution.program,
        )
    }
    pub fn claim_matches(
        &self,
        output: Option<&[u64]>,
        input: Option<&[u64]>,
        budget: u64,
    ) -> bool {
        output.is_none_or(|v| v == self.statement.execution.public_output)
            && input.is_none_or(|v| v == self.statement.execution.public_input)
            && self.statement.execution.budget <= budget
    }
    pub fn has_header(path: &Path) -> bool {
        crate::file_input::has_header(path, MAGIC, MAX_BYTES)
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let mut bytes = MAGIC.to_vec();
        bytes.extend(
            postcard::to_allocvec(self).map_err(|_| "private artifact serialization failed")?,
        );
        if bytes.len() > MAX_BYTES {
            return Err("private artifact exceeds size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("private artifact exceeds size limit".into());
        }
        let payload = bytes
            .strip_prefix(MAGIC)
            .ok_or("not a private execution artifact")?;
        let (artifact, rest): (Self, &[u8]) =
            postcard::take_from_bytes(payload).map_err(|_| "invalid private execution artifact")?;
        if !rest.is_empty() {
            return Err("trailing private artifact data".into());
        }
        artifact.validate()?;
        Ok(artifact)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::from_bytes(&crate::file_input::read(path, MAX_BYTES)?)
    }
    pub fn save(&self, path: &Path) -> Result<usize, String> {
        let bytes = self.to_bytes()?;
        fs::write(path, &bytes).map_err(|e| e.to_string())?;
        Ok(bytes.len())
    }
    pub fn proof_data(&self) -> Result<ProofData, String> {
        Ok(ProofData {
            format: ZK_EXECUTION_FORMAT.into(),
            proof_bytes: self.to_bytes()?,
            claim: trident::field::proof::Claim {
                program_hash: crate::program_hash(&self.assembly)
                    .chunks_exact(8)
                    .map(|b| {
                        let mut limb = [0u8; 8];
                        limb.copy_from_slice(b);
                        u64::from_le_bytes(limb)
                    })
                    .collect(),
                public_input: self.statement.execution.public_input.clone(),
                public_output: self.statement.execution.public_output.clone(),
            },
        })
    }
}

impl Warrior {
    pub fn prove_zk_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        budget: u64,
    ) -> Result<(ZkExecutionArtifact, ExecutionResult), String> {
        require_backend()?;
        if bundle.target_vm != "nox" || bundle.reads_state || !input.digests.is_empty() {
            return Err("private CCS execution currently requires stateless nox inputs".into());
        }
        let program = parse_program(&bundle.assembly)?;
        let (statement, prepared, witness) = zheng::execution::private::prepare_execution(
            &program,
            &input.public,
            &input.secret,
            budget,
        )?;
        let native = self
            .run(bundle, input)
            .map_err(|_| "private native execution failed")?;
        if native.output != statement.execution.public_output
            || native.cycle_count != statement.execution.cycles
        {
            return Err("private relation disagrees with native nox".into());
        }
        let mut artifact = ZkExecutionArtifact {
            format: ZK_EXECUTION_FORMAT.into(),
            program: bundle.name.clone(),
            assembly: bundle.assembly.clone(),
            source_hash: bundle.source_hash.clone(),
            statement,
            proof: vec![],
            state: None,
            root_in_subject: false,
        };
        artifact.validate()?;
        let checker = Checker::new(
            &prepared.relation.instance,
            &prepared.public_coordinates,
            &artifact.binding(),
        )?;
        artifact.proof = trisha_rs::ccs::encode_proof(&checker.prove(&witness)?)?;
        Ok((artifact, native))
    }
}

pub fn verify_proof_data(proof: &ProofData) -> Result<bool, String> {
    if proof.format != ZK_EXECUTION_FORMAT {
        return Err("unsupported private proof format".into());
    }
    let artifact = ZkExecutionArtifact::from_bytes(&proof.proof_bytes)?;
    if artifact.proof_data()?.claim != proof.claim {
        return Ok(false);
    }
    Ok(artifact.verify().is_ok())
}

#[path = "zk_state.rs"]
mod state;
use state::state_tables;
