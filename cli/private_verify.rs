//! Native private proofs verify their public statement without witness or replay.
use crate::{load_bundle, verify::VerifyArgs};
use joy_rs::ZkExecutionArtifact;

pub fn try_verify(args: &VerifyArgs) -> Option<Result<(), String>> {
    let path = args.proof.as_ref().unwrap_or(&args.input);
    if !ZkExecutionArtifact::has_header(path) {
        return None;
    }
    Some(ZkExecutionArtifact::load(path).and_then(|artifact| verify(args, &artifact)))
}

fn verify(args: &VerifyArgs, artifact: &ZkExecutionArtifact) -> Result<(), String> {
    if args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err("private proof verification uses only public inputs and expected state".into());
    }
    if args.legacy_trace_statement {
        return Err("native private proofs require ordinary execution verification".into());
    }
    if !artifact.claim_matches(
        args.claim.as_deref(),
        args.input_values.as_deref(),
        args.budget,
    ) {
        return Err("private proof differs from requested input, output or budget".into());
    }
    if args.proof.is_some() {
        let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
        if !artifact.matches_program(&bundle)? {
            return Err("private proof is for another program or compilation identity".into());
        }
    }
    if let Some(path) = &args.state {
        let expected = joy_rs::state_execution::load_certificate(std::path::Path::new(path))?;
        let actual = artifact.state.as_ref().ok_or("proof has no state")?;
        if expected.root()? != actual.root()? {
            return Err("private proof state root mismatch".into());
        }
    }
    artifact.verify()?;
    println!("Verification: PASS (native private execution; Zheng)");
    println!(
        "  public input: {:?}",
        artifact.statement.execution.public_input
    );
    println!(
        "  public output: {:?}",
        artifact.statement.execution.public_output
    );
    println!("  reductions: {}", artifact.statement.execution.cycles);
    Ok(())
}
