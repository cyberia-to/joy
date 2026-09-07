//! proof — the zheng proof artifact: what `joy prove` writes and
//! `joy verify --proof` reads.
//!
//! Wire form is JSON via zheng's serde feature (Goldilocks = canonical
//! u64, non-canonical values rejected at deserialize). The statement is
//! the cryptographic payload; `meta` is operational context (program
//! name, executed outputs, cycle count) — the outputs are bound to the
//! proof only through `statement.output_hash` (the hemera hash of the
//! last trace row), not individually.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use zheng::{Statement, TraceProof};

/// Proof system identifier written into every artifact.
pub const PROOF_FORMAT: &str = "zheng-hypernova-v1";

/// A zheng proof artifact: statement + proof + operational metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProofArtifact {
    /// Proof system identifier (must equal [`PROOF_FORMAT`]).
    pub format: String,
    /// Public statement the proof attests to.
    pub statement: Statement,
    /// The zheng trace proof (per-CCS-structure accumulator groups).
    pub proof: TraceProof,
    /// Operational context — NOT individually proof-bound.
    pub meta: ArtifactMeta,
}

/// Operational metadata carried alongside the proof.
#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactMeta {
    /// Bundle name the proof was generated from.
    pub program: String,
    /// Executed output values (bound in aggregate via statement.output_hash).
    pub output: Vec<u64>,
    /// Trace rows consumed (= reductions = budget spent).
    pub cycle_count: u64,
}

impl ProofArtifact {
    /// Write the artifact as JSON. Returns the byte length written.
    pub fn save(&self, path: &Path) -> Result<usize, String> {
        let json = serde_json::to_vec(self)
            .map_err(|e| format!("cannot serialize proof artifact: {}", e))?;
        fs::write(path, &json)
            .map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
        Ok(json.len())
    }

    /// Read an artifact back. Rejects unknown formats and malformed wire
    /// data (including non-canonical field elements).
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let artifact: ProofArtifact = serde_json::from_slice(&bytes)
            .map_err(|e| format!("malformed proof artifact {}: {}", path.display(), e))?;
        if artifact.format != PROOF_FORMAT {
            return Err(format!(
                "unknown proof format '{}' (this joy verifies '{}')",
                artifact.format, PROOF_FORMAT
            ));
        }
        Ok(artifact)
    }
}

/// hemera hash of a bundle's formula text — `Statement.program_hash`.
///
/// Hashes the trimmed bracket-notation assembly, so the same formula
/// always names the same program regardless of surrounding whitespace.
pub fn program_hash(assembly: &str) -> [u8; 32] {
    *hemera::hash(assembly.trim().as_bytes()).as_bytes()
}
