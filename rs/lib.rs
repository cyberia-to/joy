//! joy-rs — CPU backend for the joy warrior.
//!
//! Implements Trident's `Runner`/`Prover`/`Verifier`/`Deployer` traits
//! over the nox VM (cyber-nox `reduce()`). The boundary object is
//! `trident::runtime::ProgramBundle`; the `assembly` field carries the
//! `.nox` formula in bracket notation.

pub mod execution;
mod file_input;
pub mod zk_execution;
pub use file_input::read_program_text;
pub mod formula;
pub mod proof;
pub mod structured;
pub mod target;
pub mod warrior;
pub mod worker;

pub use proof::program_hash;
pub use target::{nox_terrain, target_package};
pub use warrior::{Warrior, DEFAULT_BUDGET};

pub use execution::{ExecutionArtifact, EXECUTION_FORMAT};
pub use zk_execution::{ZkExecutionArtifact, ZkScheme, ZK_EXECUTION_FORMAT};

pub mod state_execution;
pub mod machine_execution;
pub mod key_cache;
pub mod memory;
pub mod recursive_execution;
pub mod succinct_execution;
pub mod wrapped_execution;
pub use machine_execution::{MachineExecutionArtifact, MACHINE_EXECUTION_FORMAT};
pub use recursive_execution::{RecursiveExecutionArtifact, RECURSIVE_EXECUTION_FORMAT};
pub use wrapped_execution::{WrappedExecutionArtifact, WRAPPED_EXECUTION_FORMAT};
pub use succinct_execution::{
    Fallback, SuccinctArtifact, SuccinctExecutionArtifact, SUCCINCT_EXECUTION_FORMAT,
};
pub use state_execution::{StateExecutionArtifact, STATE_EXECUTION_FORMAT};
