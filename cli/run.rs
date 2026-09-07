use std::path::PathBuf;
use std::process;

use clap::Args;

use joy_rs::Warrior;
use trident::runtime::Runner;

use super::{check_target, load_bundle, make_input};

#[derive(Args)]
pub struct RunArgs {
    /// Input: .json bundle, .tri source (or project dir), or raw .nox formula
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
    /// Reduction budget (bounds trace rows one-to-one)
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Chain state (accepted for trident delegation; no chain wiring yet)
    #[arg(long)]
    pub state: Option<String>,
}

pub fn cmd_run(args: RunArgs) {
    if let Err(e) = check_target(&args.target) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
    if let Some(ref s) = args.state {
        eprintln!("note: state '{}' ignored (chain wiring lands after M4)", s);
    }
    let bundle = match load_bundle(&args.input, &args.profile) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };
    let pi = make_input(&args.input_values, &args.secret);
    let warrior = Warrior::with_budget(args.budget);
    match warrior.run(&bundle, &pi) {
        Ok(result) => {
            for val in &result.output {
                println!("{}", val);
            }
            eprintln!("Executed in {} reductions", result.cycle_count);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    }
}
