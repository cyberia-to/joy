use std::path::PathBuf;
use std::process;
use std::time::Instant;

use clap::Args;

use joy_rs::Warrior;

use super::{check_target, load_bundle};

#[derive(Args)]
pub struct ProveArgs {
    /// Input: .json bundle, .tri source, or raw .nox formula
    pub input: PathBuf,
    /// Target terrain (nox) or battlefield (cyber)
    #[arg(long)]
    pub target: Option<String>,
    /// Compilation profile for .tri inputs (debug or release)
    #[arg(long, default_value = "debug")]
    pub profile: String,
    /// Public input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub input_values: Option<Vec<u64>>,
    /// Secret/divine input values (comma-separated field elements)
    #[arg(long, value_delimiter = ',')]
    pub secret: Option<Vec<u64>>,
    /// Native public/secret input file (JSON); conflicts with values on the CLI
    #[arg(long, conflicts_with_all = ["input_values", "secret"])]
    pub input_file: Option<PathBuf>,
    /// Use native Zheng private proving even when no secret inputs are supplied
    #[arg(long)]
    pub zk: bool,
    /// Prove secret inputs with linear MPC-in-the-head instead of zheng's
    /// succinct zk scheme (veil); the fallback and the differential oracle
    #[arg(long)]
    pub mith: bool,
    /// Commit the witness instead of disclosing it (zheng envelope profile 1)
    #[arg(long, conflicts_with_all = ["zk", "secret", "state", "mith"])]
    pub succinct: bool,
    /// With --succinct: prove a program beyond one relation with the
    /// accumulated machine (profile 4) instead of recursion (profiles 5/6)
    #[arg(long, requires = "succinct", conflicts_with_all = ["wrap", "no_wrap"])]
    pub machine: bool,
    /// With --succinct: wrap a program beyond one relation (profile 6,
    /// ≤ 64 KB) whatever memory is available (the default when enough is)
    #[arg(long, requires = "succinct", conflicts_with = "no_wrap")]
    pub wrap: bool,
    /// With --succinct: prove a program beyond one relation recursively
    /// (profile 5) without the wrap chain
    #[arg(long, requires = "succinct")]
    pub no_wrap: bool,
    /// Reduction budget (also the statement's focus bound)
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    /// Artifact path (default: <input stem>.zheng next to the input)
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Replace an existing output only after proving succeeds
    #[arg(long)]
    pub force: bool,
    /// Public BBG state certificate (JSON)
    #[arg(long)]
    pub state: Option<String>,
}

/// Default artifact path: `<input stem>.zheng` next to the input.
fn artifact_path(input: &std::path::Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    input.with_file_name(format!("{}.zheng", stem))
}

pub fn cmd_prove(args: ProveArgs) {
    if let Err(e) = check_target(args.target.as_deref().unwrap_or("nox")) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
    let bundle = match load_bundle(&args.input, &args.profile, args.target.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };
    let pi = match crate::input_file::resolve(
        &args.input_values,
        &args.secret,
        args.input_file.as_deref(),
    ) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(1);
        }
    };
    let private = args.zk || args.mith || !pi.secret.is_empty();
    let scheme = if args.mith {
        joy_rs::ZkScheme::Mith
    } else {
        joy_rs::ZkScheme::Veil
    };
    if args.succinct && private {
        eprintln!("error: --succinct proves public statements; secret inputs need --zk");
        process::exit(1);
    }
    let warrior = Warrior::with_budget(args.budget);
    let fallback = if args.machine {
        joy_rs::Fallback::Machine
    } else if args.no_wrap {
        joy_rs::Fallback::Recursive
    } else if args.wrap {
        joy_rs::Fallback::Wrapped
    } else {
        match joy_rs::memory::wrap_fits() {
            (true, _) => joy_rs::Fallback::Wrapped,
            (false, available) => {
                if args.succinct {
                    let gib = |b: u64| b as f64 / (1u64 << 30) as f64;
                    let have = available.map_or("unknown".to_string(), |a| format!("{:.1} GiB", gib(a)));
                    eprintln!(
                        "note: a program beyond one relation is proven recursively (profile 5, ~285 KB): wrapping it to ≤ 64 KB (profile 6) needs ~{:.0} GiB of memory, {have} available; --wrap forces it",
                        gib(joy_rs::memory::WRAP_PEAK_BYTES)
                    );
                }
                joy_rs::Fallback::Recursive
            }
        }
    };

    let t0 = Instant::now();
    let path = args.output.unwrap_or_else(|| artifact_path(&args.input));
    let result = if let Some(state_path) = &args.state {
        joy_rs::state_execution::load_certificate(std::path::Path::new(state_path)).and_then(
            |certificate| {
                if private {
                    warrior
                        .prove_zk_state_certificate(&bundle, &pi, &certificate, args.budget)
                        .and_then(|(artifact, result)| {
                            artifact.to_bytes().map(|bytes| (result, bytes))
                        })
                } else {
                    warrior
                        .prove_state_certificate(&bundle, &pi, &certificate, args.budget)
                        .and_then(|(artifact, result)| {
                            artifact.to_bytes().map(|bytes| (result, bytes))
                        })
                }
            },
        )
    } else if args.succinct {
        warrior
            .prove_succinct_with(&bundle, &pi, args.budget, fallback)
            .and_then(|(artifact, result)| artifact.to_bytes().map(|bytes| (result, bytes)))
    } else if private {
        warrior
            .prove_zk_execution_with(&bundle, &pi, args.budget, scheme)
            .and_then(|(artifact, result)| artifact.to_bytes().map(|bytes| (result, bytes)))
    } else {
        warrior
            .prove_execution(&bundle, &pi, args.budget)
            .and_then(|(artifact, result)| artifact.to_bytes().map(|bytes| (result, bytes)))
    };
    let (result, bytes) = match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(1);
        }
    };
    if let Err(error) = crate::publication::atomic_write(&path, &bytes, args.force) {
        eprintln!("error: {error}");
        process::exit(1);
    }
    // the prover holds the verifying keys of what it proved: cache them for
    // later `joy verify` processes
    if args.succinct {
        if let Some(profile) = bytes.get(10).and_then(|&b| joy_rs::key_cache::profile_of(b)) {
            if let Err(error) = joy_rs::key_cache::store(profile) {
                eprintln!("note: verifying keys not cached ({error})");
            }
        }
    }
    let mode = if private && args.mith {
        "private execution (zheng MPC-in-the-head)"
    } else if private {
        "private execution (zheng zk, veil where the relation admits it)"
    } else if args.state.is_some() {
        "authenticated public state execution"
    } else if args.succinct {
        match bytes.get(10) {
            Some(6) => "succinct public execution (wrapped, envelope profile 6; constant size, ≤ 64 KB)",
            Some(5) => "succinct public execution (recursive, envelope profile 5; constant size)",
            Some(4) => "succinct public execution (accumulated machine, envelope profile 4)",
            _ => "succinct public execution (committed witness, one relation, envelope profile 1)",
        }
    } else {
        "public execution"
    };
    eprintln!(
        "Proved {mode} in {} ms: {} reductions, {} bytes",
        t0.elapsed().as_millis(),
        result.cycle_count,
        bytes.len()
    );
    eprintln!("Output: {:?}", result.output);
    println!("{}", path.display());
}
