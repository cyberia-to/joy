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
    /// Reduction budget (also the statement's focus bound)
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Artifact path (default: <input stem>.zheng next to the input)
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Chain state (not yet supported; requests fail explicitly)
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
    if let Err(e) = check_target(&args.target) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
    let bundle = match load_bundle(&args.input, &args.profile) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };
    if args.state.is_some() {
        // bbg has no whole-state file loader yet (storage is dimension-level
        // KV behind optional features). The library path is wired:
        // joy_rs::Warrior::prove_zheng_with_state(bundle, input, &BbgState).
        eprintln!("error: --state file loading is not wired: bbg has no state-file format yet");
        eprintln!("authenticated state execution proofs are not implemented");
        process::exit(1);
    }
    let pi = make_input(&args.input_values, &args.secret);
    let warrior = Warrior::with_budget(args.budget);

    let t0 = Instant::now();
    let (artifact, result) = match warrior.prove_execution(&bundle, &pi, args.budget) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };
    let prove_ms = t0.elapsed().as_millis();

    let path = args.output.unwrap_or_else(|| artifact_path(&args.input));
    let bytes = match artifact.save(&path) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    // stdout: machine-readable artifact location; stderr: the story.
    eprintln!(
        "Proved public execution in {} ms: {} reductions, {} bytes",
        prove_ms, result.cycle_count, bytes
    );
    eprintln!("Output: {:?}", result.output);
    println!("{}", path.display());
}
