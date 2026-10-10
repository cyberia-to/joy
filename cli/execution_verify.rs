//! Public execution verification precedes legacy trace-statement compatibility.
use crate::{load_bundle, verify::VerifyArgs};
use joy_rs::{
    execution::LegacyExecutionArtifact, ExecutionArtifact, MachineExecutionArtifact,
    key_cache::{self, KeySource, Profile},
    RecursiveExecutionArtifact, SuccinctExecutionArtifact, WrappedExecutionArtifact,
};

pub fn try_verify(args: &VerifyArgs) -> Option<Result<(), String>> {
    let path = args.proof.as_ref().unwrap_or(&args.input);
    if joy_rs::execution::has_unsupported_private_header(path) {
        return Some(Err(
            "retired foreign private proof format; regenerate with native Zheng proving".into(),
        ));
    }
    if LegacyExecutionArtifact::has_header(path) {
        return Some(LegacyExecutionArtifact::load(path).and_then(|artifact| {
            artifact.verify()?;
            println!("Verification: PASS (public execution v2, retired format; re-prove for v3)");
            println!("  public output: {:?}", artifact.statement.public_output);
            Ok(())
        }));
    }
    if SuccinctExecutionArtifact::has_header(path) {
        return Some(
            SuccinctExecutionArtifact::load(path).and_then(|artifact| verify_succinct(args, &artifact)),
        );
    }
    if MachineExecutionArtifact::has_header(path) {
        return Some(
            MachineExecutionArtifact::load(path).and_then(|artifact| verify_machine(args, &artifact)),
        );
    }
    if RecursiveExecutionArtifact::has_header(path) {
        return Some(with_keys(Profile::Recursive, || {
            RecursiveExecutionArtifact::load(path).and_then(|artifact| verify_recursive(args, &artifact))
        }));
    }
    if WrappedExecutionArtifact::has_header(path) {
        return Some(with_keys(Profile::Wrapped, || {
            WrappedExecutionArtifact::load(path).and_then(|artifact| verify_wrapped(args, &artifact))
        }));
    }
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
        let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
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

fn verify_succinct(args: &VerifyArgs, artifact: &SuccinctExecutionArtifact) -> Result<(), String> {
    if args.state.is_some() || args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err("succinct execution verification takes no state or secret inputs".into());
    }
    if !artifact.claim_matches(
        args.claim.as_deref(),
        args.input_values.as_deref(),
        args.budget,
    ) {
        return Err("execution proof differs from requested input, output or budget".into());
    }
    if args.proof.is_some() {
        let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
        if bundle.target_vm != "nox"
            || bundle.reads_state
            || !artifact.matches_program(&bundle.assembly)?
        {
            return Err("execution proof is for another program".into());
        }
    }
    artifact.verify()?;
    println!("Verification: PASS (succinct public execution; committed witness, non-ZK)");
    println!("  public input: {:?}", artifact.statement.public_input);
    println!("  public output: {:?}", artifact.statement.public_output);
    println!("  reductions: {}", artifact.statement.cycles);
    Ok(())
}

fn verify_machine(args: &VerifyArgs, artifact: &MachineExecutionArtifact) -> Result<(), String> {
    if args.state.is_some() || args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err("machine execution verification takes no state or secret inputs".into());
    }
    if !artifact.claim_matches(
        args.claim.as_deref(),
        args.input_values.as_deref(),
        args.budget,
    ) {
        return Err("execution proof differs from requested input, output or budget".into());
    }
    if args.proof.is_some() {
        let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
        if bundle.target_vm != "nox"
            || bundle.reads_state
            || !artifact.matches_program(&bundle.assembly)?
        {
            return Err("execution proof is for another program".into());
        }
    }
    artifact.verify()?;
    println!("Verification: PASS (machine public execution; uniform step relation, accumulated, non-ZK)");
    println!("  public input: {:?}", artifact.statement.input);
    println!("  public output: {:?}", artifact.output()?);
    println!("  reductions: {}", artifact.statement.cycles);
    Ok(())
}

fn verify_recursive(args: &VerifyArgs, artifact: &RecursiveExecutionArtifact) -> Result<(), String> {
    if args.state.is_some() || args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err("recursive execution verification takes no state or secret inputs".into());
    }
    if !artifact.claim_matches(
        args.claim.as_deref(),
        args.input_values.as_deref(),
        args.budget,
    ) {
        return Err("execution proof differs from requested input, output or budget".into());
    }
    if args.proof.is_some() {
        let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
        if bundle.target_vm != "nox"
            || bundle.reads_state
            || !artifact.matches_program(&bundle.assembly)?
        {
            return Err("execution proof is for another program".into());
        }
    }
    artifact.verify()?;
    println!("Verification: PASS (recursive public execution; IVC over the nox machine, non-ZK)");
    println!("  public input: {:?}", artifact.statement.input);
    println!("  public output: {:?}", artifact.output()?);
    println!("  reductions: {}", artifact.statement.cycles);
    Ok(())
}

/// Install the cached verifying keys of `profile` before decoding (profiles
/// 5 and 6 decode under their keys), run `f`, and cache keys the verifier
/// had to derive.
fn with_keys(profile: Profile, f: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let source = key_cache::load(profile);
    let result = f();
    match &source {
        KeySource::Cache(path) => eprintln!("keys: {}", path.display()),
        KeySource::Derive if result.is_ok() => match key_cache::store(profile) {
            Ok(path) => eprintln!("keys: derived, cached at {}", path.display()),
            Err(error) => eprintln!("keys: derived, not cached ({error})"),
        },
        _ => {}
    }
    result
}

fn verify_wrapped(args: &VerifyArgs, artifact: &WrappedExecutionArtifact) -> Result<(), String> {
    if args.state.is_some() || args.secret.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err("wrapped execution verification takes no state or secret inputs".into());
    }
    if !artifact.claim_matches(
        args.claim.as_deref(),
        args.input_values.as_deref(),
        args.budget,
    ) {
        return Err("execution proof differs from requested input, output or budget".into());
    }
    if args.proof.is_some() {
        let bundle = load_bundle(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
        if bundle.target_vm != "nox"
            || bundle.reads_state
            || !artifact.matches_program(&bundle.assembly)?
        {
            return Err("execution proof is for another program".into());
        }
    }
    artifact.verify()?;
    println!("Verification: PASS (wrapped public execution; IVC over the nox machine, final verifier wrapped, non-ZK)");
    println!("  public input: {:?}", artifact.statement.input);
    println!("  public output: {:?}", artifact.output()?);
    println!("  reductions: {}", artifact.statement.cycles);
    Ok(())
}
