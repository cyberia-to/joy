use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct PackArgs {
    #[arg(long)]
    pub compiler: PathBuf,
    #[arg(long)]
    pub manifest: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(long)]
    pub force: bool,
    #[command(flatten)]
    pub limits: crate::artifact_limits::LimitArgs,
}

fn pack(args: PackArgs) -> Result<serde_json::Value, String> {
    let packed =
        joy_rs::structured::pack_job_files(&args.compiler, &args.manifest, args.limits.values())?;
    crate::publication::atomic_write(&args.output, &packed.bytes, args.force)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"schema":"joy/job-pack/v1", "ok":true,
        "artifact":args.output.to_string_lossy(), "package":packed.report}))
}

pub fn cmd_pack(args: PackArgs) {
    match pack(args) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
