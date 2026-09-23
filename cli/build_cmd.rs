use crate::error::JoyError;
use crate::publication::atomic_write;
use clap::{Args, ValueEnum};
use std::path::PathBuf;
#[derive(Clone, Copy, ValueEnum)]
pub enum Emit {
    Bundle,
    Nox,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Human,
    JsonV1,
}
#[derive(Args)]
pub struct BuildArgs {
    /// Trident source or project directory
    pub input: PathBuf,
    /// Explicit nox/cyber target, overriding the project target
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "debug")]
    pub profile: String,
    #[arg(long, value_enum, default_value = "bundle")]
    pub emit: Emit,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Atomically replace an existing output file
    #[arg(long)]
    pub force: bool,
    #[arg(long, value_enum, default_value = "human")]
    pub format: Format,
}
fn build(args: &BuildArgs) -> Result<(PathBuf, crate::compile::CompiledSource), JoyError> {
    if !args.input.is_dir() && args.input.extension().and_then(|v| v.to_str()) != Some("tri") {
        return Err(JoyError::Compile(
            "build expects .tri source or project directory".into(),
        ));
    }
    let compiled = crate::compile::compile_source_with_metadata(
        &args.input,
        &args.profile,
        args.target.as_deref(),
    )?;
    let bundle = &compiled.bundle;
    let suffix = match args.emit {
        Emit::Bundle => "bundle.json",
        Emit::Nox => "nox",
    };
    let output = args.output.clone().unwrap_or_else(|| {
        if args.input.is_dir() {
            args.input.join(format!("{}.{}", bundle.name, suffix))
        } else {
            args.input.with_extension(suffix)
        }
    });
    let bytes = match args.emit {
        Emit::Bundle => bundle.to_json().into_bytes(),
        Emit::Nox => bundle.assembly.as_bytes().to_vec(),
    };
    atomic_write(&output, &bytes, args.force)?;
    Ok((output, compiled))
}
pub fn cmd_build(args: BuildArgs) {
    match build(&args) {
        Ok((path, compiled)) => match args.format {
            Format::Human => println!("{}", path.display()),
            Format::JsonV1 => println!(
                "{}",
                serde_json::json!({
                    "schema":"joy/cli/v1", "command":"build", "ok":true, "error":null,
                    "result": {
                        "kind":"build", "artifact":path.to_string_lossy(),
                        "format":match args.emit { Emit::Bundle => "bundle", Emit::Nox => "nox" },
                        "program":compiled.bundle.name, "source_hash":compiled.bundle.source_hash,
                        "target":compiled.target, "target_vm":compiled.bundle.target_vm,
                        "target_os":compiled.bundle.target_os, "profile":args.profile,
                        "reads_state":compiled.bundle.reads_state, "entry_point":compiled.bundle.entry_point,
                        "compiler":{"name":"trident", "version":trident::COMPILER_VERSION, "api":compiled.compiler_api},
                        "target_package":{"owner":compiled.package_owner,"version":compiled.package_version,"compilation_hash":compiled.package_hash}
                    }
                })
            ),
        },
        Err(error) => {
            match args.format {
                Format::Human => eprintln!("error: {error}"),
                Format::JsonV1 => println!(
                    "{}",
                    serde_json::json!({
                        "schema":"joy/cli/v1", "command":"build", "ok":false, "result":null,
                        "error":{"code":match &error { JoyError::Compile(_) => "compile_failed", JoyError::Io(_) => "artifact_write_failed", _ => "build_failed" },
                            "message":error.to_string(), "retryable":false}
                    })
                ),
            }
            std::process::exit(1);
        }
    }
}
