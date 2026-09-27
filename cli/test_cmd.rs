//! Native test execution uses Trident's checked AST and bounded nox test runner.
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct TestArgs {
    /// Trident source or project containing #[test] functions
    pub input: PathBuf,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "debug")]
    pub profile: String,
}

fn run(args: TestArgs) -> Result<String, String> {
    let resolved =
        crate::compile::resolve_source(&args.input, &args.profile, args.target.as_deref())
            .map_err(|e| e.to_string())?;
    trident::run_tests(&resolved.entry, &resolved.options)
        .map_err(|errors| crate::compile::diagnostics(errors).to_string())
}

pub fn cmd_test(args: TestArgs) {
    match run(args) {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
