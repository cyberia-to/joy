//! Complete ART1/raw-noun execution with bounded pure nox and no retained trace.
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use serde::Serialize;
use std::{
    path::Path,
    time::{Duration, Instant},
};

const ARENA: usize = 1 << 18;
const STACK: usize = 256 << 20;
const ART1: u64 = 0x41525431;

#[derive(Debug, Clone, Copy)]
pub struct RunLimits {
    pub budget: u64,
    pub arena_nodes: u32,
    pub frames: u32,
    pub artifact_bytes: usize,
    pub artifact_nodes: u32,
    pub artifact_depth: u32,
    pub time_ms: u64,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            budget: 1_000_000,
            arena_nodes: 196_608,
            frames: 16_384,
            artifact_bytes: 16 << 20,
            artifact_nodes: 196_608,
            artifact_depth: 4096,
            time_ms: 30_000,
        }
    }
}

impl RunLimits {
    pub fn validate(self) -> Result<(), String> {
        for (name, value, max) in [
            ("budget", self.budget, 100_000_000),
            ("arena_nodes", self.arena_nodes as u64, 196_608),
            ("frames", self.frames as u64, 65_536),
            ("artifact_bytes", self.artifact_bytes as u64, 16 << 20),
            ("artifact_nodes", self.artifact_nodes as u64, 196_608),
            ("artifact_depth", self.artifact_depth as u64, 4096),
            ("time_ms", self.time_ms, 60_000),
        ] {
            if value == 0 || value > max {
                return Err(format!("limit {name} must be in 1..={max}"));
            }
        }
        Ok(())
    }

    fn transport(self) -> artifact::Limits {
        artifact::Limits {
            max_bytes: self.artifact_bytes,
            max_nodes: self.artifact_nodes,
            max_depth: self.artifact_depth,
        }
    }
}

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
    pub worker_stack_bytes: usize,
    pub elapsed_micros: u128,
    pub trace_mode: &'static str,
}

#[derive(Debug)]
pub struct RunResult {
    pub output: Vec<u8>,
    pub report: RunReport,
}

fn particle(ar: &Reduction<ARENA>, root: Order) -> Result<String, String> {
    let digest = ar.digest(root).ok_or("missing particle")?;
    Ok(nox::data::digest_bytes(digest)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

// Exact ART1 record arity, target and raw-noun profiles. The guest job profile
// requires additional binding/admission and is deliberately not admitted here.
fn program_formula(ar: &Reduction<ARENA>, root: Order) -> Result<Order, String> {
    let tag = ar
        .head(root)
        .and_then(|n| ar.atom_value(n))
        .map(|v| v.as_u64());
    if tag != Some(ART1) {
        return Err("program must be ART1".into());
    }
    let mut cursor = ar.tail(root).ok_or("ART1 fields")?;
    let mut fields = [0; 4];
    for field in &mut fields {
        *field = ar.head(cursor).ok_or("ART1 arity")?;
        cursor = ar.tail(cursor).ok_or("ART1 arity")?;
    }
    if ar.atom_value(cursor).map(|v| v.as_u64()) != Some(0) {
        return Err("ART1 terminator".into());
    }
    for field in &fields[..3] {
        if ar.atom_value(*field).map(|v| v.as_u64()) != Some(0) {
            return Err("ART1 requires machine0 and raw input/output profiles(0,0)".into());
        }
    }
    Ok(fields[3])
}

fn deadline(start: Instant, limits: RunLimits) -> Result<(), String> {
    if start.elapsed() >= Duration::from_millis(limits.time_ms) {
        Err("execution deadline exceeded".into())
    } else {
        Ok(())
    }
}

fn execute(program: Vec<u8>, input: Vec<u8>, limits: RunLimits) -> Result<RunResult, String> {
    let started = Instant::now();
    let mut ar = Reduction::<ARENA>::new();
    if !ar.limit_allocations(limits.arena_nodes) {
        return Err("arena allowance rejected".into());
    }
    let transport = limits.transport();
    let program = artifact::decode(&mut ar, &program, transport)
        .map_err(|e| format!("program artifact: {e:?}"))?;
    let formula = program_formula(&ar, program)?;
    deadline(started, limits)?;
    let input = artifact::decode(&mut ar, &input, transport)
        .map_err(|e| format!("input artifact: {e:?}"))?;
    deadline(started, limits)?;
    let frames = sequential::Limits {
        max_frames: limits.frames,
    };
    let execution = sequential::reduce_controlled(
        &mut ar,
        input,
        formula,
        limits.budget,
        frames,
        &mut NoTrace,
        &mut || started.elapsed() >= Duration::from_millis(limits.time_ms),
    )
    .map_err(|e| format!("execution resource/profile: {e:?}"))?;
    let (result, remaining) = match execution.outcome {
        Outcome::Ok(result, remaining) => (result, remaining),
        Outcome::Halt(_) => return Err("execution budget exhausted".into()),
        Outcome::Error(error) => return Err(format!("execution failed: {error:?}")),
    };
    deadline(started, limits)?;
    let output =
        artifact::encode(&ar, result, transport).map_err(|e| format!("output artifact: {e:?}"))?;
    deadline(started, limits)?;
    Ok(RunResult {
        output,
        report: RunReport {
            program_particle: particle(&ar, program)?,
            input_particle: particle(&ar, input)?,
            output_particle: particle(&ar, result)?,
            charged_reductions: limits.budget - remaining,
            allocated_nodes: ar.count(),
            peak_frames: execution.peak_frames,
            arena_reserved_bytes: std::mem::size_of::<Reduction<ARENA>>(),
            frame_buffer_bytes: sequential::frame_storage_bytes(frames)
                .ok_or("frame size overflow")?,
            worker_stack_bytes: STACK,
            elapsed_micros: started.elapsed().as_micros(),
            trace_mode: "none",
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
        .spawn(move || execute(program, input, limits))
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
