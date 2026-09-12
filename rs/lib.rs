//! joy-rs — CPU backend for the joy warrior.
//!
//! Implements Trident's `Runner`/`Prover`/`Verifier`/`Deployer` traits
//! over the nox VM (cyber-nox `reduce()`). The boundary object is
//! `trident::runtime::ProgramBundle`; the `assembly` field carries the
//! `.nox` formula in bracket notation.

pub mod execution;
pub mod formula;
pub mod proof;
pub mod target;
pub mod warrior;

pub use proof::{program_hash, ArtifactMeta, ProofArtifact, PROOF_FORMAT};
pub use target::{nox_terrain, target_package};
pub use warrior::{Warrior, DEFAULT_BUDGET};

pub use execution::{ExecutionArtifact, EXECUTION_FORMAT};
