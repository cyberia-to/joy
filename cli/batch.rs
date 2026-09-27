//! Isolated native jobs with bounded concurrency and deterministic reporting.
use clap::{Args, Subcommand};
use std::{
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MAX_JOBS: usize = 256;
const MAX_OUTPUT: usize = 4 * 1024 * 1024;

#[derive(Args)]
pub struct BatchArgs {
    #[command(subcommand)]
    pub operation: Operation,
}
#[derive(Subcommand)]
pub enum Operation {
    /// Execute native programs in isolated processes
    Run(RunArgs),
    /// Prove native programs into separate numbered directories
    Prove(ProveArgs),
    /// Verify independent native proof artifacts
    Verify(VerifyArgs),
}
#[derive(Args)]
pub struct Jobs {
    #[arg(required = true, num_args = 1..)]
    pub inputs: Vec<PathBuf>,
    /// Maximum simultaneous processes (1..8)
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=8))]
    pub max_parallel: u8,
}
#[derive(Args)]
pub struct Execution {
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value = "debug")]
    pub profile: String,
    #[arg(long, value_delimiter = ',', conflicts_with = "input_file")]
    pub input_values: Option<Vec<u64>>,
    /// Private input contents are passed by file, never copied into child argv
    #[arg(long, conflicts_with = "input_values")]
    pub input_file: Option<PathBuf>,
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
    #[arg(long)]
    pub state: Option<PathBuf>,
}
#[derive(Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub jobs: Jobs,
    #[command(flatten)]
    pub execution: Execution,
}
#[derive(Args)]
pub struct ProveArgs {
    #[command(flatten)]
    pub jobs: Jobs,
    #[command(flatten)]
    pub execution: Execution,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub zk: bool,
    /// Permit replacement of existing numbered proof outputs
    #[arg(long)]
    pub force: bool,
}
#[derive(Args)]
pub struct VerifyArgs {
    #[command(flatten)]
    pub jobs: Jobs,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long, default_value_t = joy_rs::DEFAULT_BUDGET)]
    pub budget: u64,
}
struct Job {
    index: usize,
    args: Vec<OsString>,
}
struct ResultOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn flag(args: &mut Vec<OsString>, name: &str, value: impl AsRef<std::ffi::OsStr>) {
    args.push(name.into());
    args.push(value.as_ref().into());
}
impl Execution {
    fn options(&self) -> Result<Vec<OsString>, String> {
        crate::input_file::resolve(&self.input_values, &None, self.input_file.as_deref())?;
        let mut args = Vec::new();
        if let Some(target) = &self.target {
            crate::check_target(target).map_err(|e| e.to_string())?;
            flag(&mut args, "--target", target);
        }
        flag(&mut args, "--profile", &self.profile);
        flag(&mut args, "--budget", self.budget.to_string());
        if let Some(values) = &self.input_values {
            flag(
                &mut args,
                "--input-values",
                values
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        if let Some(path) = &self.input_file {
            flag(&mut args, "--input-file", path);
        }
        if let Some(path) = &self.state {
            flag(&mut args, "--state", path);
        }
        Ok(args)
    }
}
fn directory(path: &Path, force: bool) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && force => {
            let metadata =
                fs::symlink_metadata(path).map_err(|_| "cannot inspect batch output directory")?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("batch output must be a directory without symbolic links".into());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err("batch output cannot be a reparse point".into());
                }
            }
            Ok(())
        }
        Err(_) => {
            Err("cannot create batch output directory; --force permits existing directories".into())
        }
    }
}
fn prepare(operation: Operation) -> Result<(Vec<Job>, usize), String> {
    let (verb, jobs, options, output) = match operation {
        Operation::Run(a) => ("run", a.jobs, a.execution.options()?, None),
        Operation::Prove(a) => {
            let mut options = a.execution.options()?;
            if a.zk {
                options.push("--zk".into());
            }
            if a.force {
                options.push("--force".into());
            }
            ("prove", a.jobs, options, Some((a.output, a.force)))
        }
        Operation::Verify(a) => {
            let mut options = Vec::new();
            if let Some(target) = a.target {
                crate::check_target(&target).map_err(|e| e.to_string())?;
                flag(&mut options, "--target", target);
            }
            flag(&mut options, "--budget", a.budget.to_string());
            ("verify", a.jobs, options, None)
        }
    };
    if jobs.inputs.is_empty()
        || jobs.inputs.len() > MAX_JOBS
        || !(1..=8).contains(&jobs.max_parallel)
    {
        return Err("batch requires 1..256 inputs and 1..8 parallel processes".into());
    }
    // Canonicalize inputs before any output preparation, rejecting ambiguous
    // option-looking names without passing a caller-controlled flag to a child.
    let inputs = jobs
        .inputs
        .iter()
        .map(|path| {
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| "cannot inspect a batch input".to_string())?;
            if !metadata.is_file() && !metadata.is_dir() {
                return Err("batch inputs must be regular files or project directories".into());
            }
            fs::canonicalize(path).map_err(|_| "cannot resolve a batch input".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some((path, force)) = &output {
        directory(path, *force)?;
    }
    let mut prepared = Vec::new();
    for (index, input) in inputs.into_iter().enumerate() {
        let mut args = vec![OsString::from(verb), input.into_os_string()];
        args.extend(options.iter().cloned());
        if let Some((path, force)) = &output {
            let child = path.join(format!("{index:04}"));
            directory(&child, *force)?;
            flag(&mut args, "--output", child.join("proof.zheng"));
        }
        prepared.push(Job { index, args });
    }
    Ok((prepared, usize::from(jobs.max_parallel)))
}
fn capture(mut stream: impl Read) -> Result<(Vec<u8>, bool), String> {
    let mut bytes = Vec::new();
    let mut exceeded = false;
    let mut buffer = [0u8; 8192];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|_| "cannot read batch child output")?;
        if count == 0 {
            return Ok((bytes, exceeded));
        }
        let keep = count.min(MAX_OUTPUT.saturating_sub(bytes.len()));
        bytes.extend_from_slice(&buffer[..keep]);
        exceeded |= keep != count;
    }
}
fn execute(executable: &Path, args: &[OsString]) -> Result<ResultOutput, String> {
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "cannot start batch child")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("batch child stdout unavailable")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("batch child stderr unavailable")?;
    std::thread::scope(|scope| {
        let out = scope.spawn(|| capture(stdout));
        let err = scope.spawn(|| capture(stderr));
        let status = child.wait().map_err(|_| "cannot wait for batch child")?;
        let (stdout, out_limit) = out.join().map_err(|_| "batch stdout reader failed")??;
        let (mut stderr, err_limit) = err.join().map_err(|_| "batch stderr reader failed")??;
        if out_limit || err_limit {
            stderr.extend_from_slice(b"\nerror: batch output exceeds 4 MiB per stream\n");
        }
        Ok(ResultOutput {
            success: status.success() && !out_limit && !err_limit,
            stdout,
            stderr,
        })
    })
}
fn run(args: BatchArgs) -> Result<(), String> {
    let (jobs, parallel) = prepare(args.operation)?;
    let executable =
        std::env::current_exe().map_err(|_| "cannot resolve installed Joy executable")?;
    let mut failures = 0;
    for chunk in jobs.chunks(parallel) {
        let results = std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|job| {
                    let executable = &executable;
                    (
                        job.index,
                        scope.spawn(move || execute(executable, &job.args)),
                    )
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|(index, handle)| {
                    (
                        index,
                        handle
                            .join()
                            .unwrap_or_else(|_| Err("batch worker failed".into())),
                    )
                })
                .collect::<Vec<_>>()
        });
        for (index, result) in results {
            match result {
                Ok(output) => {
                    std::io::stdout()
                        .write_all(&output.stdout)
                        .map_err(|_| "cannot write batch output")?;
                    std::io::stderr()
                        .write_all(&output.stderr)
                        .map_err(|_| "cannot write batch diagnostics")?;
                    eprintln!("[{index}] {}", if output.success { "PASS" } else { "FAIL" });
                    failures += usize::from(!output.success);
                }
                Err(error) => {
                    eprintln!("[{index}] FAIL: {error}");
                    failures += 1;
                }
            }
        }
    }
    if failures == 0 {
        Ok(())
    } else {
        Err(format!("{failures} batch job(s) failed"))
    }
}
pub fn cmd_batch(args: BatchArgs) {
    if let Err(error) = run(args) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
