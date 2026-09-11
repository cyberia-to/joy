use std::path::PathBuf;
use std::process;

use clap::Args;

use joy_rs::Warrior;

use super::{check_target, load_bundle, make_input};

#[derive(Args)]
pub struct VerifyArgs {
    /// Input: a .zheng.json proof artifact (self-contained), or a .json
    /// bundle / .tri source / raw .nox formula (with --proof or --claim)
    pub input: PathBuf,
    /// Claimed output values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub claim: Option<Vec<u64>>,
    /// Verify a zheng proof artifact (no re-execution)
    #[arg(long)]
    pub proof: Option<PathBuf>,
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

    // A proof artifact given directly (what `trident verify <artifact>`
    // sends through the warrior boundary): self-contained verification.
    if args.proof.is_none() {
        if let Ok(artifact) = joy_rs::ProofArtifact::load(&args.input) {
            let warrior = Warrior::with_budget(args.budget);
            match warrior.verify_artifact(&artifact) {
                Ok(true) => {
                    if let Err(e) = joy_rs::proof::require_statement_only(args.claim.as_deref()) {
                        eprintln!("error: {}", e);
                        process::exit(1);
                    }
                    println!("Verification: PASS (zheng trace statement)");
                    println!("  program: {}", artifact.meta.program);
                    println!("  reported output (unverified): {:?}", artifact.meta.output);
                    println!(
                        "  reported cycles (unverified): {}",
                        artifact.meta.cycle_count
                    );
                }
                Ok(false) => {
                    println!(
                        "Verification: FAIL (zheng proof rejected — assembly or proof tampered)"
                    );
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
            }
            return;
        }
    }

    let bundle = match load_bundle(&args.input, &args.profile) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // Proof mode: verify the zheng artifact against this bundle. No
    // re-execution — the proof carries the whole trace commitment.
    if let Some(proof_path) = args.proof {
        let artifact = match joy_rs::ProofArtifact::load(&proof_path) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        };
        let warrior = Warrior::with_budget(args.budget);
        match warrior.verify_zheng(&bundle, &artifact) {
            Ok(true) => {
                if let Err(e) = joy_rs::proof::require_statement_only(args.claim.as_deref()) {
                    eprintln!("error: {}", e);
                    process::exit(1);
                }
                println!("Verification: PASS (zheng trace statement)");
                println!("  program: {}", artifact.meta.program);
                println!("  reported output (unverified): {:?}", artifact.meta.output);
                println!(
                    "  reported cycles (unverified): {}",
                    artifact.meta.cycle_count
                );
            }
            Ok(false) => {
                println!("Verification: FAIL (zheng proof rejected for this bundle)");
                process::exit(1);
            }
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

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
