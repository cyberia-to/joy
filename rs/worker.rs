//! Typed verification boundary for a caller's saved computation job.
//!
//! The caller owns scheduling, trusted roots, persistence and admission policy.
//! This module checks computation; job IDs do not establish proof freshness.
use crate::{ExecutionArtifact, StateExecutionArtifact, ZkExecutionArtifact};
use trident::runtime::ProgramBundle;
use zheng::execution::ExecutionStatement;

const MAX_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofProfile {
    Public,
    PublicState,
    Private,
    PrivateState,
}
impl ProofProfile {
    pub fn format(self) -> &'static str {
        match self {
            Self::Public => crate::EXECUTION_FORMAT,
            Self::PublicState => crate::STATE_EXECUTION_FORMAT,
            Self::Private | Self::PrivateState => crate::ZK_EXECUTION_FORMAT,
        }
    }
    fn state(self) -> bool {
        matches!(self, Self::PublicState | Self::PrivateState)
    }
    fn header(self) -> &'static [u8] {
        match self {
            Self::Public => b"JOYEXEC2",
            Self::PublicState => b"JOYST001",
            Self::Private | Self::PrivateState => b"JOYZH001",
        }
    }
}

/// Expectations supplied by the scheduler, independently of the worker result.
/// Private witness material has no field in this structure.
#[derive(Clone, Debug)]
pub struct JobSpec {
    pub job_id: [u8; 32],
    pub attempt: u32,
    pub profile: ProofProfile,
    pub bundle: ProgramBundle,
    pub public_input: Vec<u64>,
    pub expected_output: Option<Vec<u64>>,
    pub budget: u64,
    pub max_artifact_bytes: usize,
    pub expected_state_root: Option<[u64; 4]>,
}

/// Owns an immutable admitted specification. Construct before worker dispatch.
#[derive(Clone, Debug)]
pub struct ExpectedJob(JobSpec);
impl ExpectedJob {
    pub fn new(spec: JobSpec) -> Result<Self, WorkerError> {
        let canonical = |values: &[u64]| values.iter().all(|&v| v < nebu::field::P);
        if spec.bundle.target_vm != "nox"
            || spec
                .bundle
                .target_os
                .as_deref()
                .is_some_and(|os| os != "cyber")
            || (!spec.profile.state() && spec.bundle.reads_state)
        {
            return Err(WorkerError::UnsupportedProfile);
        }
        if spec.budget == 0 || spec.max_artifact_bytes == 0 || spec.max_artifact_bytes > MAX_BYTES {
            return Err(WorkerError::InvalidLimits);
        }
        if spec.public_input.len() > 64
            || !canonical(&spec.public_input)
            || spec
                .expected_output
                .as_ref()
                .is_some_and(|v| v.len() > 4096 || !canonical(v))
            || spec.profile.state() != spec.expected_state_root.is_some()
            || spec
                .expected_state_root
                .as_ref()
                .is_some_and(|v| !canonical(v))
            || crate::execution::parse_program(&spec.bundle.assembly).is_err()
        {
            return Err(WorkerError::InvalidJob);
        }
        Ok(Self(spec))
    }
    pub fn spec(&self) -> &JobSpec {
        &self.0
    }
}

/// Untrusted transport envelope. All computation coordinates come from the
/// verified artifact, never from separate worker-reported output or cost.
pub struct ProducedProof<'a> {
    pub job_id: [u8; 32],
    pub attempt: u32,
    pub profile: ProofProfile,
    pub artifact: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerError {
    InvalidJob,
    UnsupportedProfile,
    InvalidLimits,
    JobMismatch,
    AttemptMismatch,
    ProfileMismatch,
    ArtifactTooLarge,
    ProofRejected,
    ProgramMismatch,
    InputMismatch,
    OutputMismatch,
    BudgetExceeded,
    StateMismatch,
}
impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidJob => "invalid native computation job",
            Self::UnsupportedProfile => "unsupported native target/profile combination",
            Self::InvalidLimits => "invalid computation resource limits",
            Self::JobMismatch => "result belongs to another job",
            Self::AttemptMismatch => "result belongs to another attempt",
            Self::ProfileMismatch => "result proof profile differs from the saved job",
            Self::ArtifactTooLarge => "result artifact exceeds the saved byte limit",
            Self::ProofRejected => "computation proof was rejected",
            Self::ProgramMismatch => "verified program differs from the saved bundle",
            Self::InputMismatch => "verified public inputs differ from the saved job",
            Self::OutputMismatch => "verified public outputs differ from the saved expectation",
            Self::BudgetExceeded => "verified budget exceeds the saved limit",
            Self::StateMismatch => "verified state root differs from the saved trusted root",
        })
    }
}
impl std::error::Error for WorkerError {}

/// Created only after verification and comparison with independently saved
/// expectations. This carries no network acceptance, reward or finality claim.
#[derive(Clone, Debug)]
pub struct VerifiedExecution {
    job_id: [u8; 32],
    attempt: u32,
    profile: ProofProfile,
    program_particle: [u8; 32],
    public_input: Vec<u64>,
    public_output: Vec<u64>,
    reductions: u64,
    statement_budget: u64,
    state_root: Option<[u64; 4]>,
}
impl VerifiedExecution {
    pub fn job_id(&self) -> [u8; 32] {
        self.job_id
    }
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    pub fn profile(&self) -> ProofProfile {
        self.profile
    }
    /// Hemera of the expected exact nox assembly after trimming.
    pub fn program_particle(&self) -> [u8; 32] {
        self.program_particle
    }
    pub fn public_input(&self) -> &[u64] {
        &self.public_input
    }
    pub fn public_output(&self) -> &[u64] {
        &self.public_output
    }
    pub fn reductions(&self) -> u64 {
        self.reductions
    }
    pub fn statement_budget(&self) -> u64 {
        self.statement_budget
    }
    pub fn state_root(&self) -> Option<[u64; 4]> {
        self.state_root
    }
}

/// Verify without executing the program, consulting the worker or loading keys.
/// Byte limits and dispatch metadata are checked before artifact decoding.
pub fn verify_result(
    job: &ExpectedJob,
    result: ProducedProof<'_>,
) -> Result<VerifiedExecution, WorkerError> {
    let expected = job.spec();
    if result.job_id != expected.job_id {
        return Err(WorkerError::JobMismatch);
    }
    if result.attempt != expected.attempt {
        return Err(WorkerError::AttemptMismatch);
    }
    if result.profile != expected.profile || !result.artifact.starts_with(expected.profile.header())
    {
        return Err(WorkerError::ProfileMismatch);
    }
    if result.artifact.len() > expected.max_artifact_bytes {
        return Err(WorkerError::ArtifactTooLarge);
    }
    let (statement, root, matches) = match expected.profile {
        ProofProfile::Public => {
            let artifact = ExecutionArtifact::from_bytes(result.artifact)
                .map_err(|_| WorkerError::ProofRejected)?;
            artifact.verify().map_err(|_| WorkerError::ProofRejected)?;
            let matches = artifact.assembly == expected.bundle.assembly
                && artifact.program == expected.bundle.name
                && artifact
                    .matches_program(&expected.bundle.assembly)
                    .map_err(|_| WorkerError::ProgramMismatch)?;
            (artifact.statement, None, matches)
        }
        ProofProfile::PublicState => {
            let artifact = StateExecutionArtifact::from_bytes(result.artifact)
                .map_err(|_| WorkerError::ProofRejected)?;
            artifact.verify().map_err(|_| WorkerError::ProofRejected)?;
            let matches = artifact.assembly == expected.bundle.assembly
                && artifact.program == expected.bundle.name
                && artifact
                    .matches_program(&expected.bundle)
                    .map_err(|_| WorkerError::ProgramMismatch)?;
            (
                artifact.statement.execution,
                Some(artifact.statement.state_root),
                matches,
            )
        }
        ProofProfile::Private | ProofProfile::PrivateState => {
            let artifact = ZkExecutionArtifact::from_bytes(result.artifact)
                .map_err(|_| WorkerError::ProofRejected)?;
            if artifact.state.is_some() != expected.profile.state() {
                return Err(WorkerError::ProfileMismatch);
            }
            artifact.verify().map_err(|_| WorkerError::ProofRejected)?;
            let root = artifact
                .state
                .as_ref()
                .map(|s| s.root())
                .transpose()
                .map_err(|_| WorkerError::ProofRejected)?;
            let matches = artifact.assembly == expected.bundle.assembly
                && artifact
                    .matches_program(&expected.bundle)
                    .map_err(|_| WorkerError::ProgramMismatch)?;
            (artifact.statement.execution, root, matches)
        }
    };
    compare(expected, statement, root, matches)
}
fn compare(
    expected: &JobSpec,
    statement: ExecutionStatement,
    root: Option<[u64; 4]>,
    matches: bool,
) -> Result<VerifiedExecution, WorkerError> {
    if !matches {
        return Err(WorkerError::ProgramMismatch);
    }
    if statement.public_input != expected.public_input {
        return Err(WorkerError::InputMismatch);
    }
    if expected
        .expected_output
        .as_ref()
        .is_some_and(|v| v != &statement.public_output)
    {
        return Err(WorkerError::OutputMismatch);
    }
    if statement.budget > expected.budget || statement.cycles > expected.budget {
        return Err(WorkerError::BudgetExceeded);
    }
    if root != expected.expected_state_root {
        return Err(WorkerError::StateMismatch);
    }
    Ok(VerifiedExecution {
        job_id: expected.job_id,
        attempt: expected.attempt,
        profile: expected.profile,
        program_particle: crate::program_hash(&expected.bundle.assembly),
        public_input: statement.public_input,
        public_output: statement.public_output,
        reductions: statement.cycles,
        statement_budget: statement.budget,
        state_root: root,
    })
}
