//! Public native structured certificates; specs/structured-certificates.md.
use super::{
    RunLimits, ARENA, COMPILER_ARENA, DEFAULT_ARENA_NODES, LARGE_ARENA, LARGE_ARENA_NODES, STACK,
};
use std::{
    io::{Read, Write},
    path::Path,
};

mod admission;
mod capture;
mod pipeline;
mod records;
mod session;
pub mod transport;
mod types;
pub use capture::CaptureStats as ProverObservations;
pub use pipeline::{CertificateResult, Report, FORMAT};
pub use types::CertificateLimits;
use types::Context;
#[cfg(test)]
mod tests;

/// Produce a disclosed certificate, returning the fully flushed output owner.
pub fn prove<W: Write + Send + 'static>(
    program: Vec<u8>,
    input: Vec<u8>,
    output: W,
    host: RunLimits,
    limits: CertificateLimits,
) -> Result<(W, CertificateResult), String> {
    host.validate()?;
    limits.validate()?;
    worker(move || {
        if host.resident_nodes() > LARGE_ARENA_NODES {
            pipeline::prove::<COMPILER_ARENA, W>(&program, &input, output, host, limits)
        } else if host.resident_nodes() > DEFAULT_ARENA_NODES {
            pipeline::prove::<LARGE_ARENA, W>(&program, &input, output, host, limits)
        } else {
            pipeline::prove::<ARENA, W>(&program, &input, output, host, limits)
        }
    })
}

/// Verify against explicit expected artifacts, independently of nox execution.
pub fn verify<R: Read + Send + 'static>(
    program: Vec<u8>,
    input: Vec<u8>,
    proof: R,
    host: RunLimits,
    limits: CertificateLimits,
) -> Result<CertificateResult, String> {
    host.validate()?;
    limits.validate()?;
    worker(move || {
        if host.resident_nodes() > LARGE_ARENA_NODES {
            pipeline::verify::<COMPILER_ARENA, R>(&program, &input, proof, host, limits)
        } else if host.resident_nodes() > DEFAULT_ARENA_NODES {
            pipeline::verify::<LARGE_ARENA, R>(&program, &input, proof, host, limits)
        } else {
            pipeline::verify::<ARENA, R>(&program, &input, proof, host, limits)
        }
    })
}

pub fn prove_files<W: Write + Send + 'static>(
    program: &Path,
    input: &Path,
    output: W,
    host: RunLimits,
    limits: CertificateLimits,
) -> Result<(W, CertificateResult), String> {
    host.validate()?;
    limits.validate()?;
    let program = crate::file_input::read(program, host.artifact_bytes)?;
    let input = crate::file_input::read(input, host.artifact_bytes)?;
    prove(program, input, output, host, limits)
}
pub fn verify_files(
    program: &Path,
    input: &Path,
    proof: &Path,
    host: RunLimits,
    limits: CertificateLimits,
) -> Result<CertificateResult, String> {
    host.validate()?;
    limits.validate()?;
    let program = crate::file_input::read(program, host.artifact_bytes)?;
    let input = crate::file_input::read(input, host.artifact_bytes)?;
    let proof = crate::file_input::open(
        proof,
        usize::try_from(limits.wire_bytes).map_err(|_| "proof byte limit exceeds platform")?,
    )?;
    verify(program, input, proof, host, limits)
}

fn worker<T: Send + 'static>(
    run: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    std::thread::Builder::new()
        .name("joy-certificate".into())
        .stack_size(STACK)
        .spawn(run)
        .map_err(|e| format!("certificate worker spawn: {e}"))?
        .join()
        .map_err(|_| "certificate worker panicked".to_string())?
}
