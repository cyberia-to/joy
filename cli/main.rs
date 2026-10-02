mod artifact_limits;
mod artifact_run;
mod batch;
mod bench;
mod build_cmd;
mod certificate;
mod compile;
mod error;
mod execution_verify;
mod input_file;
mod pack_job;
mod private_verify;
mod prove;
mod publication;
mod raw_build;
mod run;
mod state_verify;
mod test_cmd;
mod verify;

use clap::{Parser, Subcommand};

use crate::error::JoyError;
use trident::runtime::ProgramInput;

#[derive(Parser)]
#[command(
    name = "joy",
    version,
    about = "nox warrior — execute, prove, verify on the cyber battlefield"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Describe the installed target package without executing a program (JSON)
    Describe {
        #[arg(long, default_value = "nox")]
        target: String,
    },
    /// Compile a source or project to a metadata-preserving bundle or nox assembly
    Build(build_cmd::BuildArgs),
    /// Execute independent runs, proofs or verifications with bounded concurrency
    Batch(batch::BatchArgs),
    /// Execute compiled native #[test] functions
    Test(test_cmd::TestArgs),
    /// Measure native execution of an explicit program and input
    Bench(bench::BenchArgs),
    /// Execute a Trident program on the nox VM
    Run(run::RunArgs),
    /// Execute an ART1 program on a complete raw noun without retaining a trace
    RunArtifact(artifact_run::RunArgs),
    /// Prove complete pure ART1 execution with a disclosed native certificate
    ProveArtifact(certificate::ProveArgs),
    /// Verify a disclosed ART1 certificate without running nox or a compiler
    VerifyArtifact(certificate::VerifyArgs),
    /// Package exact source files into JOB1 without compiling them
    PackJob(pack_job::PackArgs),
    /// Execute and generate a zheng proof artifact
    Prove(prove::ProveArgs),
    /// Verify a zheng proof (--proof) or a claimed output by re-execution (--claim)
    Verify(verify::VerifyArgs),
}

pub(crate) fn make_input(
    input_values: &Option<Vec<u64>>,
    secret: &Option<Vec<u64>>,
) -> ProgramInput {
    ProgramInput {
        public: input_values.clone().unwrap_or_default(),
        secret: secret.clone().unwrap_or_default(),
        digests: Vec::new(),
    }
}

/// joy answers for the nox terrain and the cyber battlefield only.
pub(crate) fn check_target(target: &str) -> Result<(), JoyError> {
    match target {
        "nox" | "cyber" => Ok(()),
        t => Err(JoyError::Execute(format!(
            "joy supports only soft3 targets nox and cyber; unsupported target '{}'",
            t
        ))),
    }
}

/// Load a ProgramBundle from any of joy's accepted inputs:
/// `.json` bundle, `.tri` source (or project dir), or raw `.nox` formula.
pub(crate) fn load_bundle(
    input: &std::path::Path,
    profile: &str,
    target: Option<&str>,
) -> Result<trident::runtime::ProgramBundle, JoyError> {
    if let Some(target) = target {
        check_target(target)?;
    }
    if input.is_dir() {
        return compile::compile_source(input, profile, target);
    }
    match input.extension().and_then(|e| e.to_str()) {
        Some("json") => {
            let text = joy_rs::read_program_text(input)
                .map_err(|e| JoyError::Io(format!("cannot read '{}': {}", input.display(), e)))?;
            trident::runtime::ProgramBundle::from_json(&text).map_err(JoyError::Parse)
        }
        Some("tri") => compile::compile_source(input, profile, target),
        Some("nox") => bundle_from_nox(input),
        _ => Err(JoyError::Io(format!(
            "unsupported input '{}' (expected .json bundle, .tri source, or .nox formula)",
            input.display()
        ))),
    }
}

/// Wrap a raw .nox formula file into a minimal bundle.
fn bundle_from_nox(path: &std::path::Path) -> Result<trident::runtime::ProgramBundle, JoyError> {
    let assembly = joy_rs::read_program_text(path)
        .map_err(|e| JoyError::Io(format!("cannot read '{}': {}", path.display(), e)))?;
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let source_hash = trident::hash::content_hash_bytes(assembly.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(trident::runtime::ProgramBundle {
        name,
        version: String::new(),
        target_vm: "nox".to_string(),
        target_os: None,
        assembly,
        entry_point: String::new(),
        functions: Vec::new(),
        cost: trident::runtime::artifact::BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 0,
            estimated_proving_ns: 0,
        },
        source_hash,
        reads_state: false,
    })
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Describe { target } => match joy_rs::target::describe(&target) {
            Ok(description) => println!("{description}"),
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        },
        Command::Build(args) => build_cmd::cmd_build(args),
        Command::Batch(args) => batch::cmd_batch(args),
        Command::Test(args) => test_cmd::cmd_test(args),
        Command::Bench(args) => bench::cmd_bench(args),
        Command::Run(args) => run::cmd_run(args),
        Command::RunArtifact(args) => artifact_run::cmd_run(args),
        Command::ProveArtifact(args) => certificate_result(certificate::prove(args)),
        Command::VerifyArtifact(args) => certificate_result(certificate::verify(args)),
        Command::PackJob(args) => pack_job::cmd_pack(args),
        Command::Prove(args) => prove::cmd_prove(args),
        Command::Verify(args) => verify::cmd_verify(args),
    }
}

fn certificate_result(result: Result<serde_json::Value, JoyError>) {
    match result {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
