//! proof — the program identity every joy proof artifact binds.
//!
//! The retired trace-statement artifact (`zheng-hypernova-tensor-merkle-v2`,
//! `ProofArtifact`) lived here. It rode zheng's legacy folded trace API,
//! which binds no statement and checks no fold (zheng specs/soundness.md,
//! retired), so it authenticated nothing and was removed. Public, private
//! and state proofs live in `execution`, `zk_execution` and
//! `state_execution`.

/// hemera hash of a bundle's formula text — the program identity in a
/// proof claim.
///
/// Hashes the trimmed bracket-notation assembly, so the same formula
/// always names the same program regardless of surrounding whitespace.
pub fn program_hash(assembly: &str) -> [u8; 32] {
    *hemera::hash(assembly.trim().as_bytes()).as_bytes()
}
