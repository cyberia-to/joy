use super::*;
use bbg::certificate::StateCertificate;
use zheng::execution::state_evidence::StateEvidence;
/// The certificate as zheng's evidence; zheng authenticates it under the
/// statement's root (leaves → root, tables → leaves, all ten namespaces)
/// before the tables become relation constants.
pub(super) fn state_evidence(certificate: &StateCertificate) -> Result<StateEvidence, String> {
    certificate.evidence()
}
impl Warrior {
    pub fn prove_zk_state_execution(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        state: &bbg::BbgState,
        budget: u64,
    ) -> Result<(ZkExecutionArtifact, ExecutionResult), String> {
        let certificate = StateCertificate::from_state(state, &(0..10).collect::<Vec<_>>())?;
        self.prove_zk_state_certificate(bundle, input, &certificate, budget)
    }

    pub fn prove_zk_state_certificate(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        state: &StateCertificate,
        budget: u64,
    ) -> Result<(ZkExecutionArtifact, ExecutionResult), String> {
        if bundle.target_vm != "nox" || !input.digests.is_empty() {
            return Err("private state execution requires nox inputs".into());
        }
        let evidence = state_evidence(state)?;
        let program = parse_program(&bundle.assembly)?;
        let (statement, prepared, witness) = zheng::execution::private_state::prepare_execution(
            &program,
            &input.public,
            &input.secret,
            budget,
            bundle.reads_state,
            &evidence,
        )?;
        let result = self
            .run_state_certificate(bundle, input, state, budget)
            .map_err(|_| "private native state execution failed")?;
        if result.output != statement.execution.public_output
            || result.cycle_count != statement.execution.cycles
        {
            return Err("private state relation disagrees with native nox".into());
        }
        let mut artifact = ZkExecutionArtifact {
            format: ZK_EXECUTION_FORMAT.into(),
            program: bundle.name.clone(),
            assembly: bundle.assembly.clone(),
            source_hash: bundle.source_hash.clone(),
            statement: PrivateStatement {
                execution: statement.execution,
            },
            proof: vec![],
            state: Some(state.clone()),
            root_in_subject: bundle.reads_state,
        };
        artifact.validate()?;
        artifact.proof = prove_native(&prepared, &witness, &artifact.binding(), ZkScheme::Veil)?;
        Ok((artifact, result))
    }
}
