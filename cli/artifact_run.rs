use clap::{Args, ValueEnum};
use joy_rs::structured::{self, CompilerCaps, RunLimits};
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
pub enum Emit {
    Result,
    Program,
}

#[derive(Args)]
pub struct RunArgs {
    /// Complete NOXDAG01 artifact containing a raw or compiler-profile ART1 program
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
    /// Publish the complete result noun, or a successfully compiled ART1
    #[arg(long, value_enum, default_value = "result")]
    pub emit: Emit,
    #[arg(long, default_value_t = 4_194_304)]
    pub source_bytes: u32,
    #[arg(long, default_value_t = 4096)]
    pub modules: u32,
    #[arg(long, default_value_t = 1024)]
    pub diagnostics: u32,
    #[arg(long, default_value_t = 65_536)]
    pub sequence_length: u32,
    #[arg(long, default_value_t = 1_000_000)]
    pub validation_visits: u32,
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
        compiler: CompilerCaps {
            source_bytes: args.source_bytes,
            modules: args.modules,
            diagnostics: args.diagnostics,
            sequence_length: args.sequence_length,
            validation_visits: args.validation_visits,
        },
    };
    let result = structured::run_files(&args.program, &args.input, limits)?;
    let (bytes, published_particle) = match args.emit {
        Emit::Result => (&result.output, result.report.output_particle.as_str()),
        Emit::Program => {
            let job = result
                .report
                .compiler_job
                .as_ref()
                .ok_or("--emit program requires compiler profile(1,1)")?;
            let bytes = result.compiled.as_ref().ok_or_else(|| {
                let diagnostics = job
                    .diagnostics
                    .iter()
                    .map(|d| {
                        format!(
                            "code{} module{} [{}..{}): {}",
                            d.code, d.module_index, d.start_byte, d.end_byte, d.message
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("guest compilation failed: {diagnostics}")
            })?;
            (
                bytes,
                job.compiled_particle
                    .as_deref()
                    .ok_or("missing compiled particle")?,
            )
        }
    };
    crate::publication::atomic_write(&args.output, bytes, args.force).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "schema": "joy/artifact-run/v1", "ok": true,
        "artifact": args.output.to_string_lossy(), "published_particle": published_particle, "execution": result.report,
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
