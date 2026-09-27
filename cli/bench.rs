//! Measure the supplied native program; results never imply an unrun baseline.
use clap::Args;
use serde_json::{json, Value};
use std::{path::PathBuf, time::Instant};
use trident::runtime::Runner;

#[derive(Args)]
pub struct BenchArgs {
    /// Trident source/project, JSON bundle, or raw nox formula to execute
    pub input: PathBuf,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "debug")]
    pub profile: String,
    #[arg(long, value_delimiter = ',', conflicts_with = "input_file")]
    pub input_values: Option<Vec<u64>>,
    /// Canonical native input document, including private witnesses if needed
    #[arg(long, conflicts_with = "input_values")]
    pub input_file: Option<PathBuf>,
    /// Independent expected public output, checked on every run
    #[arg(long, value_delimiter = ',')]
    pub claim: Option<Vec<u64>>,
    #[arg(long)]
    pub state: Option<PathBuf>,
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Number of measured executions (1..1000), each with a fresh witness stream
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u16).range(1..=1000))]
    pub repeat: u16,
}

fn measure(args: BenchArgs) -> Result<Value, String> {
    let input = crate::input_file::resolve(&args.input_values, &None, args.input_file.as_deref())?;
    // Validate independent expectations through the same canonical field boundary.
    crate::input_file::resolve(&args.claim, &None, None)?;
    let compiled_source =
        args.input.is_dir() || args.input.extension().and_then(|e| e.to_str()) == Some("tri");
    let compile = Instant::now();
    let bundle = crate::load_bundle(&args.input, &args.profile, args.target.as_deref())
        .map_err(|e| e.to_string())?;
    let preparation_ns = compile.elapsed().as_nanos();
    let certificate = args
        .state
        .as_deref()
        .map(joy_rs::state_execution::load_certificate)
        .transpose()?;
    let warrior = joy_rs::Warrior::with_budget(args.budget);
    let mut samples = Vec::new();
    let mut times = Vec::new();
    let mut expected = None;
    for _ in 0..args.repeat {
        let start = Instant::now();
        let result = match &certificate {
            Some(state) => warrior.run_state_certificate(&bundle, &input, state, args.budget),
            None => warrior.run(&bundle, &input),
        }?;
        let elapsed = start.elapsed().as_nanos();
        if args
            .claim
            .as_ref()
            .is_some_and(|claim| claim != &result.output)
        {
            return Err("benchmark output differs from the supplied claim".into());
        }
        let observed = (result.output, result.cycle_count);
        if expected
            .as_ref()
            .is_some_and(|previous| previous != &observed)
        {
            return Err("benchmark repeated execution changed output or reduction count".into());
        }
        samples.push(json!({
            "reductions": observed.1.to_string(), "elapsed_ns": elapsed.to_string(),
        }));
        times.push(elapsed);
        expected = Some(observed);
    }
    let (output, reductions) = expected.ok_or("benchmark requires at least one execution")?;
    let total_ns: u128 = times.iter().sum();
    times.sort_unstable();
    let program = joy_rs::program_hash(&bundle.assembly)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    Ok(json!({
        "schema": "joy/bench/v1",
        "kind": "native-execution",
        "program": bundle.name,
        "program_particle": program,
        "program_particle_profile": "hemera-trimmed-nox-assembly/1",
        "source_hash": bundle.source_hash,
        "target": bundle.target_vm,
        "compilation_profile": compiled_source.then_some(args.profile),
        "repeat": args.repeat,
        "budget": args.budget.to_string(),
        "program_preparation_ns": preparation_ns.to_string(),
        "output": output.iter().map(u64::to_string).collect::<Vec<_>>(),
        "reductions": reductions.to_string(),
        "expected_output_checked": args.claim.is_some(),
        "state_root": certificate.as_ref().map(|c| c.root()).transpose()?
            .map(|root| root.map(|v| v.to_string())),
        "proof_generated": false,
        "samples": samples,
        "total_ns": total_ns.to_string(),
        "min_ns": times[0].to_string(),
        "median_ns": times[times.len() / 2].to_string(),
    }))
}

pub fn cmd_bench(args: BenchArgs) {
    match measure(args) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
