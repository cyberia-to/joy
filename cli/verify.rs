use std::path::PathBuf;
use std::process;

use clap::Args;

use joy_rs::Warrior;

use super::{check_target, load_bundle, make_input};

#[derive(Args)]
pub struct VerifyArgs {
    /// Input: .json bundle, .tri source (or project dir), or raw .nox formula
    pub input: PathBuf,
    /// Claimed output values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub claim: Option<Vec<u64>>,
    /// Target terrain (nox) or battlefield (cyber)
    #[arg(long, default_value = "nox")]
    pub target: String,
    /// Compilation profile for .tri inputs (debug or release)
    #[arg(long, default_value = "debug")]
    pub profile: String,
    /// Public input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub input_values: Option<Vec<u64>>,
    /// Secret/divine input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub secret: Option<Vec<u64>>,
    /// Reduction budget
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Chain state (accepted for trident delegation; no chain wiring yet)
    #[arg(long)]
    pub state: Option<String>,
}

pub fn cmd_verify(args: VerifyArgs) {
    if let Err(e) = check_target(&args.target) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
    // A proof file is an M4 request — answer with the honest dash.
    if matches!(
        args.input.extension().and_then(|e| e.to_str()),
        Some("toml") | Some("proof")
    ) {
        eprintln!("error: zheng proof verification lands in M4 of the soft3 release");
        eprintln!("M3 verifies by re-execution: joy verify <bundle.json> --claim <values>");
        process::exit(1);
    }
    let claim = match args.claim {
        Some(c) => c,
        None => {
            eprintln!("error: --claim <values> is required (verification by re-execution)");
            process::exit(1);
        }
    };
    let bundle = match load_bundle(&args.input, &args.profile) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };
    let pi = make_input(&args.input_values, &args.secret);
    let warrior = Warrior::with_budget(args.budget);
    match warrior.verify_by_rerun(&bundle, &pi, &claim) {
        Ok(true) => {
            println!("Verification: PASS (verified by re-execution; zheng proof arrives in M4)");
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
