use super::*;
use bbg::certificate::StateCertificate;
use nebu::Goldilocks as F;
use zheng::execution::relation::PublicStateTables;
/// Authentication happens before the table becomes verifier relation constants.
pub(super) fn state_tables(certificate: &StateCertificate) -> Result<PublicStateTables, String> {
    if certificate.dimensions.len() != 10
        || certificate
            .dimensions
            .iter()
            .map(|t| t.fields.len())
            .sum::<usize>()
            > 2048
    {
        return Err(
            "private state requires all ten public namespaces and at most 2048 fields".into(),
        );
    }
    let root = certificate.root()?;
    let dimensions = std::array::from_fn(|i| {
        certificate.dimensions[i]
            .fields
            .iter()
            .copied()
            .map(F::new)
            .collect()
    });
    Ok(PublicStateTables {
        root: root.map(F::new),
        dimensions,
    })
}
impl Warrior {
    pub fn prove_zk_state_certificate(
        &self,
        bundle: &ProgramBundle,
        input: &ProgramInput,
        state: &StateCertificate,
        budget: u64,
    ) -> Result<(ZkExecutionArtifact, ExecutionResult), String> {
        require_backend()?;
        if bundle.target_vm != "nox" || !input.digests.is_empty() {
            return Err("private state execution requires nox inputs".into());
        }
        let tables = state_tables(state)?;
        let program = parse_program(&bundle.assembly)?;
        let (statement, prepared, witness) = zheng::execution::private_state::prepare_execution(
            &program,
            &input.public,
            &input.secret,
            budget,
            bundle.reads_state,
            &tables,
        )?;
        let result = self.run_state_certificate(bundle, input, state, budget)?;
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
        let checker = Checker::new(
            &prepared.relation.instance,
            &prepared.public_coordinates,
            &artifact.binding(),
        )?;
        artifact.proof = trisha_rs::ccs::encode_proof(&checker.prove(&witness)?)?;
        Ok((artifact, result))
    }
}
