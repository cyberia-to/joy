//! proof — the zheng proof artifact: what `joy prove` writes and
//! `joy verify --proof` reads.
//!
//! Wire form is compact binary (postcard, varint-packed) over zheng's
//! serde types: Goldilocks = canonical u64, non-canonical values rejected
//! at deserialize; the prover's folded witness is never on the wire. The
//! statement is the cryptographic payload; `meta` is operational context
//! (program name, executed outputs, cycle count, the assembly) — the
//! outputs are bound to the proof only through `statement.output_hash`
//! (the hemera hash of the last trace row), not individually; the
//! assembly is bound through `statement.program_hash`. JSON artifacts
//! from 0.2.0 still load.

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
    /// The zheng trace proof: the universal Layer-1 group plus, when the
    /// program opens anything, the binding group.
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
    /// The nox formula the proof was generated from. Makes the artifact
    /// self-contained: `joy verify <artifact>` recomputes program_hash
    /// from it and checks it against the statement, so the assembly is
    /// hash-bound to the proof even though it travels in `meta`. Absent
    /// in artifacts written before 0.2.0 — those need the bundle.
    #[serde(default)]
    pub assembly: Option<String>,
    /// The same assembly, deflate-compressed (bracket notation is highly
    /// repetitive: a depth-32 Merkle path is 6.5 KB of text, ~1 KB
    /// deflated). Written by 0.3.0+; takes precedence over `assembly`.
    #[serde(default)]
    pub assembly_deflate: Option<Vec<u8>>,
}

impl ArtifactMeta {
    /// The formula this proof was generated from, whichever form it travels in.
    pub fn assembly_text(&self) -> Result<Option<String>, String> {
        if let Some(z) = &self.assembly_deflate {
            let bytes = miniz_oxide::inflate::decompress_to_vec(z)
                .map_err(|e| format!("corrupt assembly_deflate: {:?}", e))?;
            return String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| format!("assembly is not UTF-8: {}", e));
        }
        Ok(self.assembly.clone())
    }

    /// Store the assembly deflated (drops any plain copy).
    pub fn set_assembly(&mut self, assembly: &str) {
        self.assembly = None;
        self.assembly_deflate =
            Some(miniz_oxide::deflate::compress_to_vec(assembly.as_bytes(), 9));
    }
}

impl ProofArtifact {
    /// Write the artifact as JSON. Returns the byte length written.
    pub fn save(&self, path: &Path) -> Result<usize, String> {
        let bytes = self.to_bytes()?;
        fs::write(path, &bytes)
            .map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
        Ok(bytes.len())
    }

    /// Compact binary wire form (postcard).
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        postcard::to_allocvec(self).map_err(|e| format!("cannot serialize proof artifact: {}", e))
    }

    /// Parse the binary wire form; falls back to the 0.2.0 JSON form.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        match postcard::from_bytes::<ProofArtifact>(bytes) {
            Ok(a) => Ok(a),
            Err(bin_err) => serde_json::from_slice::<ProofArtifact>(bytes).map_err(|json_err| {
                format!(
                    "malformed proof artifact: not postcard ({}) nor 0.2.0 JSON ({})",
                    bin_err, json_err
                )
            }),
        }
    }

    /// Read an artifact back. Rejects unknown formats and malformed wire
    /// data (including non-canonical field elements).
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let artifact = Self::from_bytes(&bytes)
            .map_err(|e| format!("{} ({})", e, path.display()))?;
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
