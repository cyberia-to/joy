use std::path::PathBuf;
use std::process;

use clap::Args;

#[derive(Args)]
pub struct ProveArgs {
    /// Input: .json bundle, .tri source, or raw .nox formula
    pub input: PathBuf,
    /// Target terrain (nox) or battlefield (cyber)
    #[arg(long, default_value = "nox")]
    pub target: String,
    /// Compilation profile (accepted for trident delegation)
    #[arg(long, default_value = "release")]
    pub profile: String,
    /// Public input values (accepted for trident delegation)
    #[arg(long, value_delimiter = ',')]
    pub input_values: Option<Vec<u64>>,
    /// Secret input values (accepted for trident delegation)
    #[arg(long, value_delimiter = ',')]
    pub secret: Option<Vec<u64>>,
    /// Output path (accepted for trident delegation)
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Chain state (accepted for trident delegation)
    #[arg(long)]
    pub state: Option<String>,
}

pub fn cmd_prove(_args: ProveArgs) {
    // The dash is the release note. No fake proofs.
    eprintln!("error: zheng prover lands in M4 of the soft3 release");
    eprintln!("until then: joy run (execute) and joy verify --claim (re-execution)");
    process::exit(1);
}
