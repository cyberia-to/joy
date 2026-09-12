//! Public execution verification precedes legacy trace-statement compatibility.
use crate::{load_bundle, verify::VerifyArgs};
use joy_rs::ExecutionArtifact;

pub fn try_verify(args: &VerifyArgs) -> Option<Result<(), String>> {
    let path = args.proof.as_ref().unwrap_or(&args.input);
    if !ExecutionArtifact::has_header(path) {
        return None;
    }
    Some(ExecutionArtifact::load(path).and_then(|artifact| verify(args, &artifact)))
}

fn verify(args: &VerifyArgs, artifact: &ExecutionArtifact) -> Result<(), String> {
    if args.state.is_some() || args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err("public execution verification takes no state or secret inputs".into());
    }
    if !artifact.claim_matches(
        args.claim.as_deref(),
        args.input_values.as_deref(),
        args.budget,
    ) {
        return Err("execution proof differs from requested input, output or budget".into());
    }
    if args.proof.is_some() {
        let bundle = load_bundle(&args.input, &args.profile).map_err(|e| e.to_string())?;
        if bundle.target_vm != "nox"
            || bundle.reads_state
            || !artifact.matches_program(&bundle.assembly)?
        {
            return Err("execution proof is for another program".into());
        }
    }
    artifact.verify()?;
    println!("Verification: PASS (public execution; non-ZK)");
    println!("  public input: {:?}", artifact.statement.public_input);
    println!("  public output: {:?}", artifact.statement.public_output);
    println!("  reductions: {}", artifact.statement.cycles);
    Ok(())
}
