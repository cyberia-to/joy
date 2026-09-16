use std::path::PathBuf;
use std::process;
use std::time::Instant;

use clap::Args;

use joy_rs::Warrior;

use super::{check_target, load_bundle, make_input};

#[derive(Args)]
pub struct ProveArgs {
    /// Input: .json bundle, .tri source, or raw .nox formula
    pub input: PathBuf,
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
    /// Use the Triton-backed zero-knowledge execution proof (automatic with secrets)
    #[arg(long)]
    pub zk: bool,
    /// Reduction budget (also the statement's focus bound)
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Artifact path (default: <input stem>.zheng next to the input)
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Public BBG state certificate (JSON)
    #[arg(long)]
    pub state: Option<String>,
}

/// Default artifact path: `<input stem>.zheng` next to the input.
fn artifact_path(input: &PathBuf) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    input.with_file_name(format!("{}.zheng", stem))
}

pub fn cmd_prove(args: ProveArgs) {
    if let Err(e) = check_target(args.target.as_deref().unwrap_or("nox")) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
    let bundle = match load_bundle(&args.input, &args.profile, args.target.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };
    let pi = make_input(&args.input_values, &args.secret);
    let warrior = Warrior::with_budget(args.budget);

    let t0 = Instant::now();
    let path = args.output.unwrap_or_else(|| artifact_path(&args.input));
    let zk = args.zk || !pi.secret.is_empty();
    let result = if let Some(state_path) = &args.state {
        joy_rs::state_execution::load_certificate(std::path::Path::new(state_path)).and_then(
            |certificate| {
                if zk {
                    warrior
                        .prove_zk_state_certificate(&bundle, &pi, &certificate, args.budget)
                        .and_then(|(artifact, result)| {
                            artifact.save(&path).map(|bytes| (result, bytes))
                        })
                } else {
                    warrior
                        .prove_state_certificate(&bundle, &pi, &certificate, args.budget)
                        .and_then(|(artifact, result)| {
                            artifact.save(&path).map(|bytes| (result, bytes))
                        })
                }
            },
        )
    } else if zk {
        warrior
            .prove_zk_execution(&bundle, &pi, args.budget)
            .and_then(|(artifact, result)| artifact.save(&path).map(|bytes| (result, bytes)))
    } else {
        warrior
            .prove_execution(&bundle, &pi, args.budget)
            .and_then(|(artifact, result)| artifact.save(&path).map(|bytes| (result, bytes)))
    };
    let (result, bytes) = match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(1);
        }
    };
    let mode = if args.state.is_some() && zk {
        "private authenticated state execution (Triton ZK)"
    } else if args.state.is_some() {
        "authenticated public state execution"
    } else if zk {
        "private execution (Triton ZK)"
    } else {
        "public execution"
    };
    eprintln!(
        "Proved {mode} in {} ms: {} reductions, {} bytes",
        t0.elapsed().as_millis(),
        result.cycle_count,
        bytes
    );
    eprintln!("Output: {:?}", result.output);
    println!("{}", path.display());
}
