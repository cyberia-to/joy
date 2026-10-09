//! Public state execution with complete authenticated BBG dimension evidence.
//!
//! `JOYST002` (produced): the BBG state certificate plus zheng's state-public
//! envelope (profile 3, statement + v3 certificate). `JOYST001` (retired,
//! read for one release): the same identity with a v1 direct proof.
//!
//! zheng authenticates the certificate's evidence under the statement's root
//! and every read against it; joy checks only its own identity fields. The
//! v3 statement carries no context (zheng binds no label a disclosed
//! witness could not be re-certified under); the v1 transcript absorbed
//! `context(format, program, source_hash)`, recomputed here to verify it.
use crate::{execution::parse_program, Warrior};
use bbg::{certificate::StateCertificate, BbgState};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use trident::runtime::{ExecutionResult, ProgramBundle, ProgramInput, ProofData};
use zheng::envelope::Envelope;
use zheng::execution::state::{PublicLookup, StateStatement};
use zheng::execution::{Certificate, DirectProof, ExecutionStatement};

pub const STATE_EXECUTION_FORMAT: &str = "joy-nox-public-state-execution-v3";
/// The retired v1 layout (Spartan + PublicTensor), verified for one release.
pub const LEGACY_STATE_EXECUTION_FORMAT: &str = "joy-nox-public-state-execution-v1";
pub const MAGIC: &[u8; 8] = b"JOYST002";
const LEGACY_MAGIC: &[u8; 8] = b"JOYST001";
const MAX_BYTES: usize = 64 * 1024 * 1024;

/// The execution proof inside a state artifact.
#[derive(Clone, Debug)]
pub enum StateProof {
    /// Profile v3: free witness positions, checked exactly.
    Certificate(Certificate),
    /// Retired v1 direct proof; verified, never produced.
    Direct(DirectProof),
}

#[derive(Clone, Debug)]
pub struct StateExecutionArtifact {
    pub format: String,
    pub program: String,
    pub assembly: String,
    pub source_hash: String,
    pub statement: StateStatement,
    pub certificate: StateCertificate,
    pub proof: StateProof,
}

/// `JOYST002` payload: identity, BBG certificate, then the zheng envelope.
#[derive(Serialize, Deserialize)]
struct Wire {
    format: String,
    program: String,
    assembly: String,
    source_hash: String,
    certificate: StateCertificate,
    envelope: Vec<u8>,
}

/// `JOYST001` payload, field for field the retired layout.
#[derive(Serialize, Deserialize)]
struct LegacyWire {
    format: String,
    program: String,
    assembly: String,
    source_hash: String,
    statement: LegacyStatement,
    certificate: StateCertificate,
    proof: DirectProof,
}

/// The v1 state statement, with the context it carried.
#[derive(Serialize, Deserialize)]
struct LegacyStatement {
    execution: ExecutionStatement,
    state_root: [u64; 4],
    context: [u8; 32],
    root_in_subject: bool,
    reads: Vec<PublicLookup>,
}

fn context(format: &str, program: &str, source_hash: &str) -> [u8; 32] {
    let mut h = hemera::Hasher::new();
    h.update(format.as_bytes());
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
        let expected_format = match self.proof {
            StateProof::Certificate(_) => STATE_EXECUTION_FORMAT,
            StateProof::Direct(_) => LEGACY_STATE_EXECUTION_FORMAT,
        };
        if self.format != expected_format
            || self.program.len() > 4096
            || self.source_hash.is_empty()
            || self.source_hash.len() > 128
            || ExecutionStatement::encode_program(&parse_program(&self.assembly)?)?
                != self.statement.execution.program
        {
            return Err("invalid state execution identity".into());
        }
        let evidence = self.certificate.evidence()?;
        match &self.proof {
            StateProof::Certificate(c) => self.statement.verify_certificate(c, &evidence),
            StateProof::Direct(p) => self.statement.verify_v1(&self.legacy_context(), p, &evidence),
        }
    }
    /// The context the v1 transcript absorbed.
    fn legacy_context(&self) -> [u8; 32] {
        context(LEGACY_STATE_EXECUTION_FORMAT, &self.program, &self.source_hash)
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let (mut bytes, payload) = match &self.proof {
            StateProof::Certificate(c) => {
                let envelope = Envelope::StatePublic {
                    statement: self.statement.clone(),
                    certificate: c.clone(),
                };
                let wire = Wire {
                    format: self.format.clone(),
                    program: self.program.clone(),
                    assembly: self.assembly.clone(),
                    source_hash: self.source_hash.clone(),
                    certificate: self.certificate.clone(),
                    envelope: envelope.to_bytes(),
                };
                (MAGIC.to_vec(), postcard::to_allocvec(&wire))
            }
            StateProof::Direct(p) => {
                let s = &self.statement;
                let wire = LegacyWire {
                    format: self.format.clone(),
                    program: self.program.clone(),
                    assembly: self.assembly.clone(),
                    source_hash: self.source_hash.clone(),
                    statement: LegacyStatement {
                        execution: s.execution.clone(),
                        state_root: s.state_root,
                        context: self.legacy_context(),
                        root_in_subject: s.root_in_subject,
                        reads: s.reads.clone(),
                    },
                    certificate: self.certificate.clone(),
                    proof: p.clone(),
                };
                (LEGACY_MAGIC.to_vec(), postcard::to_allocvec(&wire))
            }
        };
        bytes.extend(payload.map_err(|e| e.to_string())?);
        if bytes.len() > MAX_BYTES {
            return Err("state execution artifact size limit".into());
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("state execution artifact size limit".into());
        }
        if let Some(payload) = bytes.strip_prefix(MAGIC) {
            let (wire, rest): (Wire, &[u8]) =
                postcard::take_from_bytes(payload).map_err(|e| e.to_string())?;
            if !rest.is_empty() || wire.format != STATE_EXECUTION_FORMAT {
                return Err("invalid state execution wire format".into());
            }
            let Envelope::StatePublic {
                statement,
                certificate,
            } = Envelope::from_bytes(&wire.envelope).map_err(|e| e.to_string())?
            else {
                return Err("state artifact carries another proof profile".into());
            };
            return Ok(Self {
                format: wire.format,
                program: wire.program,
                assembly: wire.assembly,
                source_hash: wire.source_hash,
                statement,
                certificate: wire.certificate,
                proof: StateProof::Certificate(certificate),
            });
        }
        let payload = bytes
            .strip_prefix(LEGACY_MAGIC)
            .ok_or("not a state execution artifact")?;
        let (wire, rest): (LegacyWire, &[u8]) =
            postcard::take_from_bytes(payload).map_err(|e| e.to_string())?;
        if !rest.is_empty() || wire.format != LEGACY_STATE_EXECUTION_FORMAT {
            return Err("invalid state execution wire format".into());
        }
        let l = wire.statement;
        if l.context != context(LEGACY_STATE_EXECUTION_FORMAT, &wire.program, &wire.source_hash) {
            return Err("invalid state execution identity".into());
        }
        Ok(Self {
            format: wire.format,
            program: wire.program,
            assembly: wire.assembly,
            source_hash: wire.source_hash,
            statement: StateStatement {
                execution: l.execution,
                state_root: l.state_root,
                root_in_subject: l.root_in_subject,
                reads: l.reads,
            },
            certificate: wire.certificate,
            proof: StateProof::Direct(wire.proof),
        })
    }
    pub fn has_header(path: &Path) -> bool {
        crate::file_input::has_header(path, MAGIC, MAX_BYTES)
            || crate::file_input::has_header(path, LEGACY_MAGIC, MAX_BYTES)
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
            format: self.format.clone(),
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
/// The CLI state file is a public BBG certificate, not an unverified database
/// dump; its own consistency is checked here, its binding to a statement by
/// zheng.
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
        let program = parse_program(&bundle.assembly)?;
        let (statement, proof) = zheng::execution::state::certify_state_execution(
            &program,
            &input.public,
            budget,
            bundle.reads_state,
            &certificate.evidence()?,
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
            proof: StateProof::Certificate(proof),
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
    if data.format != STATE_EXECUTION_FORMAT && data.format != LEGACY_STATE_EXECUTION_FORMAT {
        return Err("unsupported state proof format".into());
    }
    let artifact = StateExecutionArtifact::from_bytes(&data.proof_bytes)?;
    Ok(artifact.proof_data()?.claim == data.claim && artifact.verify().is_ok())
}
#[path = "state_native.rs"]
mod native;
