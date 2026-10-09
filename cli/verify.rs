use std::path::PathBuf;
use std::process;

use clap::Args;

use joy_rs::Warrior;

use super::{check_target, load_bundle, make_input};

#[derive(Args)]
pub struct VerifyArgs {
    /// Input: a proof artifact (self-contained), or a .json bundle / .tri
    /// source / raw .nox formula (with --proof or --claim)
    pub input: PathBuf,
    /// Claimed output values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub claim: Option<Vec<u64>>,
    /// Verify a zheng proof artifact (no re-execution)
    #[arg(long)]
    pub proof: Option<PathBuf>,
    /// Target terrain (nox) or battlefield (cyber)
    #[arg(long)]
    pub target: Option<String>,
    /// Compilation profile for .tri inputs (debug or release)
    #[arg(long, default_value = "debug")]
    pub profile: String,
    /// Public input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub input_values: Option<Vec<u64>>,
    /// Secret/divine input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub secret: Option<Vec<u64>>,
    /// Expected public input file; secret values are accepted only for re-execution
    #[arg(long, conflicts_with_all = ["input_values", "secret"])]
    pub input_file: Option<PathBuf>,
    /// Reduction budget
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Expected public BBG state certificate (JSON)
    #[arg(long)]
    pub state: Option<String>,
}

pub fn cmd_verify(mut args: VerifyArgs) {
    if let Err(e) = check_target(args.target.as_deref().unwrap_or("nox")) {
        eprintln!("error: {}", e);
        process::exit(1);
    }

    if let Some(path) = args.input_file.as_deref() {
        match crate::input_file::resolve(&args.input_values, &args.secret, Some(path)) {
            Ok(input) => {
                args.input_values = Some(input.public);
                args.secret = if input.secret.is_empty() {
                    None
                } else {
                    Some(input.secret)
                };
            }
            Err(error) => {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
    }

    let artifact = args.proof.as_ref().unwrap_or(&args.input);
    match std::fs::symlink_metadata(artifact) {
        Ok(metadata) if metadata.is_file() || (args.proof.is_none() && metadata.is_dir()) => {}
        Ok(_) => {
            eprintln!("error: verification input must be a regular file or source project");
            process::exit(1);
        }
        Err(error) => {
            eprintln!("error: cannot inspect verification input: {error}");
            process::exit(1);
        }
    }

    if let Some(result) = crate::private_verify::try_verify(&args) {
        if let Err(error) = result {
            eprintln!("Verification: FAIL ({error})");
            process::exit(1);
        }
        return;
    }
    if let Some(result) = crate::state_verify::try_verify(&args) {
        if let Err(error) = result {
            eprintln!("Verification: FAIL ({error})");
            process::exit(1);
        }
        return;
    }
    if let Some(result) = crate::execution_verify::try_verify(&args) {
        if let Err(error) = result {
            eprintln!("Verification: FAIL ({error})");
            process::exit(1);
        }
        return;
    }

    if args.state.is_some() {
        eprintln!("error: --state requires an authenticated state execution artifact");
        process::exit(1);
    }

    // Every supported proof format was recognised above. The retired
    // trace-statement artifact (`zheng-hypernova-tensor-merkle-v2`) rode
    // zheng's legacy fold, which authenticated nothing; it is not accepted.
    if args.proof.is_some() {
        eprintln!("error: unrecognised proof artifact (legacy trace statements are retired; re-prove with `joy prove`)");
        process::exit(1);
    }

    let bundle = match load_bundle(&args.input, &args.profile, args.target.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    let claim = match args.claim {
        Some(c) => c,
        None => {
            eprintln!("error: --claim <values> (re-execution) or --proof <artifact> is required");
            process::exit(1);
        }
    };
    let pi = make_input(&args.input_values, &args.secret);
    let warrior = Warrior::with_budget(args.budget);
    match warrior.verify_by_rerun(&bundle, &pi, &claim) {
        Ok(true) => {
            println!(
                "Verification: PASS (re-execution; for a zheng proof use joy prove + --proof)"
            );
        }
        Ok(false) => {
            // Re-run once more to show the actual output in the failure report.
            let executed = trident::runtime::Runner::run(&warrior, &bundle, &pi)
                .map(|r| format!("{:?}", r.output))
                .unwrap_or_else(|e| format!("<error: {}>", e));
            println!("Verification: FAIL");
            println!("  claimed:  {:?}", claim);
            println!("  executed: {}", executed);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}
