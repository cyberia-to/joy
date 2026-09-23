use clap::Args;
use joy_rs::structured::{self, RunLimits};
use std::path::PathBuf;

#[derive(Args)]
pub struct RunArgs {
    /// Complete NOXDAG01 artifact containing a raw-profile ART1 program
    pub program: PathBuf,
    /// Complete NOXDAG01 input noun
    #[arg(long)]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    /// Atomically replace an existing output file after successful execution
    #[arg(long)]
    pub force: bool,
    #[arg(long, default_value_t = 1_000_000)]
    pub budget: u64,
    #[arg(long, default_value_t = 196_608)]
    pub arena_nodes: u32,
    #[arg(long, default_value_t = 16_384)]
    pub frames: u32,
    #[arg(long, default_value_t = 16_777_216)]
    pub artifact_bytes: usize,
    #[arg(long, default_value_t = 196_608)]
    pub artifact_nodes: u32,
    #[arg(long, default_value_t = 4096)]
    pub artifact_depth: u32,
    /// Cooperative deadline inside the execution worker
    #[arg(long, default_value_t = 30_000)]
    pub time_ms: u64,
}

fn execute(args: &RunArgs) -> Result<serde_json::Value, String> {
    let limits = RunLimits {
        budget: args.budget,
        arena_nodes: args.arena_nodes,
        frames: args.frames,
        artifact_bytes: args.artifact_bytes,
        artifact_nodes: args.artifact_nodes,
        artifact_depth: args.artifact_depth,
        time_ms: args.time_ms,
    };
    let result = structured::run_files(&args.program, &args.input, limits)?;
    crate::publication::atomic_write(&args.output, &result.output, args.force)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "schema": "joy/artifact-run/v1", "ok": true,
        "artifact": args.output.to_string_lossy(), "execution": result.report,
    }))
}

pub fn cmd_run(args: RunArgs) {
    match execute(&args) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
