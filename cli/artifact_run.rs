use clap::{Args, ValueEnum};
use joy_rs::structured;
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
    /// Publish the complete result noun, or a successfully compiled ART1
    #[arg(long, value_enum, default_value = "result")]
    pub emit: Emit,
    #[command(flatten)]
    pub limits: crate::artifact_limits::LimitArgs,
}

fn execute(args: &RunArgs) -> Result<serde_json::Value, String> {
    let limits = args.limits.values();
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
