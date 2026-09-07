//! joy-rs — CPU backend for the joy warrior.
//!
//! Implements Trident's `Runner`/`Prover`/`Verifier`/`Deployer` traits
//! over the nox VM (cyber-nox `reduce()`). The boundary object is
//! `trident::runtime::ProgramBundle`; the `assembly` field carries the
//! `.nox` formula in bracket notation.

pub mod formula;
pub mod target;
pub mod warrior;

pub use target::nox_terrain;
pub use warrior::{Warrior, DEFAULT_BUDGET};
