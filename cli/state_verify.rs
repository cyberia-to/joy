use crate::{load_bundle, verify::VerifyArgs};
use joy_rs::{state_execution::load_certificate, StateExecutionArtifact};
use std::path::Path;
pub fn try_verify(args: &VerifyArgs) -> Option<Result<(), String>> {
    let path = args.proof.as_ref().unwrap_or(&args.input);
    if !StateExecutionArtifact::has_header(path) {
        return None;
    }
    Some(StateExecutionArtifact::load(path).and_then(|artifact| {
        if args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
            return Err("public state proof verification takes no secrets".into());
        }
        let execution = &artifact.statement.execution;
        if args
            .claim
            .as_ref()
            .is_some_and(|v| v != &execution.public_output)
            || args
                .input_values
                .as_ref()
                .is_some_and(|v| v != &execution.public_input)
            || execution.budget > args.budget
        {
            return Err("state execution claim mismatch".into());
        }
        if let Some(path) = &args.state {
            let expected = load_certificate(Path::new(path))?;
            if expected.root()? != artifact.statement.state_root {
                return Err("state root differs from requested state".into());
            }
        }
        if args.proof.is_some() {
            let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
                .map_err(|e| e.to_string())?;
            if !artifact.matches_program(&bundle)? {
                return Err("state execution proof is for another build".into());
            }
        }
        artifact.verify()?;
        println!("Verification: PASS (authenticated public state execution; non-ZK)");
        println!("  public input: {:?}", execution.public_input);
        println!("  public output: {:?}", execution.public_output);
        println!("  reductions: {}", execution.cycles);
        Ok(())
    }))
}
