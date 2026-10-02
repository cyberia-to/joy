//! Complete ART1 execution with bounded pure nox and validated compiler jobs.
use nox::{artifact, sequential, Order, Reduction};
use serde::Serialize;
use std::{
    path::Path,
    time::{Duration, Instant},
};

const ARENA: usize = 1 << 18;
const LARGE_ARENA: usize = 1 << 20;
const COMPILER_ARENA: usize = 1 << 22;
const LARGE_ARENA_NODES: u32 = (LARGE_ARENA / 4 * 3) as u32;
const DEFAULT_ARENA_NODES: u32 = (ARENA / 4 * 3) as u32;
const MAX_ARENA_NODES: u32 = (COMPILER_ARENA / 4 * 3) as u32;
const MAX_BUDGET: u64 = 100_000_000;
const MAX_TIME_MS: u64 = 300_000;
const STACK: usize = 256 << 20;
#[cfg(test)]
const ART1: u64 = 0x41525431;

mod execution;
pub mod certificate;
mod job;
mod job_limits;
mod job_result;
mod limits;
mod pack;
mod pack_writer;
mod reader;
pub use execution::CompactionReport;
pub use job::{ModuleReport, Options};
pub use job_limits::{CompilerCaps, JobLimits};
pub use job_result::{CompilerReport, Diagnostic};
pub use limits::{CompactionPolicy, RunLimits};
pub use pack::{pack_job_files, PackReport, PackedJob};

#[derive(Debug, Serialize)]
pub struct RunReport {
    pub program_particle: String,
    pub input_particle: String,
    pub output_particle: String,
    pub charged_reductions: u64,
    pub allocated_nodes: u32,
    pub peak_frames: u32,
    pub arena_reserved_bytes: usize,
    pub frame_buffer_bytes: usize,
    pub finalizer_cache_bytes: usize,
    pub worker_stack_bytes: usize,
    pub elapsed_micros: u128,
    pub trace_mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_job: Option<CompilerReport>,
}

#[derive(Debug)]
pub struct RunResult {
    pub output: Vec<u8>,
    pub compiled: Option<Vec<u8>>,
    pub report: RunReport,
}

fn particle<const N: usize>(ar: &Reduction<N>, root: Order) -> Result<String, String> {
    let digest = ar.digest(root).ok_or("missing particle")?;
    Ok(nox::data::digest_bytes(digest)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn deadline(start: Instant, limits: RunLimits) -> Result<(), String> {
    if start.elapsed() >= Duration::from_millis(limits.time_ms) {
        Err("execution deadline exceeded".into())
    } else {
        Ok(())
    }
}

fn execute<const N: usize>(
    program: Vec<u8>,
    input: Vec<u8>,
    limits: RunLimits,
) -> Result<RunResult, String> {
    let started = Instant::now();
    let expires = started + Duration::from_millis(limits.time_ms);
    let mut ar = Reduction::<N>::try_new_boxed().map_err(|e| format!("arena: {e}"))?;
    if !ar.limit_allocations(limits.resident_nodes()) {
        return Err("arena allowance rejected".into());
    }
    let transport = limits.transport();
    let program = artifact::decode(&mut ar, &program, transport)
        .map_err(|e| format!("program artifact: {e:?}"))?;
    let (formula, profile, program_visits) = {
        let mut r = reader::Reader {
            ar: &ar,
            remaining: limits.compiler.validation_visits,
            sequence_limit: limits.compiler.sequence_length,
            deadline: expires,
        };
        let (formula, profile) =
            job::program(&mut r, program).map_err(|e| format!("ART1 admission: {e}"))?;
        (
            formula,
            profile,
            limits.compiler.validation_visits - r.remaining,
        )
    };
    deadline(started, limits)?;
    let input = artifact::decode(&mut ar, &input, transport)
        .map_err(|e| format!("input artifact: {e:?}"))?;
    deadline(started, limits)?;
    let admitted = if profile == 1 {
        let job = job::admit(&ar, input, program, limits, expires, program_visits)
            .map_err(|e| format!("compiler job admission: {e}"))?;
        if !ar.limit_allocations(job.limits.arena_nodes.min(limits.resident_nodes())) {
            return Err("compiler job arena allowance below loaded nodes".into());
        }
        Some(job)
    } else {
        None
    };
    let budget = admitted
        .as_ref()
        .map_or(limits.budget, |j| j.limits.reductions);
    let transport = admitted
        .as_ref()
        .map_or(transport, |j| j.limits.transport());
    let frames = sequential::Limits {
        max_frames: admitted
            .as_ref()
            .map_or(limits.frames, |j| j.limits.evaluator_frames),
    };
    let execution = execution::run(
        &mut ar,
        execution::Request {
            input,
            formula,
            budget,
            frames,
            allocations: admitted
                .as_ref()
                .map_or(limits.arena_nodes, |job| job.limits.arena_nodes),
        },
        limits,
        started,
    )?;
    let (result, remaining) = (execution.result, execution.remaining);
    deadline(started, limits)?;
    let (compiler_job, compiled) = match admitted {
        Some(job) => {
            let (report, compiled) = job_result::validate(&ar, result, job, expires)
                .map_err(|e| format!("compiler result: {e}"))?;
            (Some(report), compiled)
        }
        None => (None, None),
    };
    let output =
        artifact::encode(&ar, result, transport).map_err(|e| format!("output artifact: {e:?}"))?;
    deadline(started, limits)?;
    Ok(RunResult {
        output,
        compiled,
        report: RunReport {
            program_particle: particle(&ar, program)?,
            input_particle: particle(&ar, input)?,
            output_particle: particle(&ar, result)?,
            charged_reductions: budget - remaining,
            allocated_nodes: execution.allocated_nodes,
            peak_frames: execution.peak_frames,
            arena_reserved_bytes: std::mem::size_of::<Reduction<N>>(),
            frame_buffer_bytes: sequential::frame_storage_bytes(frames)
                .ok_or("frame size overflow")?,
            finalizer_cache_bytes: sequential::finalizer_cache_storage_bytes(),
            worker_stack_bytes: STACK,
            elapsed_micros: started.elapsed().as_micros(),
            trace_mode: "none",
            compaction: execution.compaction,
            compiler_job,
        },
    })
}

pub fn run(program: Vec<u8>, input: Vec<u8>, limits: RunLimits) -> Result<RunResult, String> {
    limits.validate()?;
    if program.len() > limits.artifact_bytes || input.len() > limits.artifact_bytes {
        return Err("input file size limit".into());
    }
    std::thread::Builder::new()
        .name("joy-artifact".into())
        .stack_size(STACK)
        .spawn(move || {
            if limits.resident_nodes() > LARGE_ARENA_NODES {
                execute::<COMPILER_ARENA>(program, input, limits)
            } else if limits.resident_nodes() > DEFAULT_ARENA_NODES {
                execute::<LARGE_ARENA>(program, input, limits)
            } else {
                execute::<ARENA>(program, input, limits)
            }
        })
        .map_err(|e| format!("worker spawn: {e}"))?
        .join()
        .map_err(|_| "artifact worker panicked".to_string())?
}

pub fn run_files(program: &Path, input: &Path, limits: RunLimits) -> Result<RunResult, String> {
    limits.validate()?;
    let program = crate::file_input::read(program, limits.artifact_bytes)?;
    let input = crate::file_input::read(input, limits.artifact_bytes)?;
    run(program, input, limits)
}

#[cfg(test)]
mod tests;
