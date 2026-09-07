//! target — the nox terrain configuration.
//!
//! Trident resolves `vm/nox/target.toml` relative to its binary or the
//! working directory. When joy runs outside the trident repo that lookup
//! fails, so joy carries a fallback mirror of the same values.
//! Sync duty: keep this in step with `trident/vm/nox/target.toml`.

use trident::target::{Arch, TerrainConfig, WarriorConfig};

/// Resolve the nox terrain config, falling back to the built-in mirror.
pub fn nox_terrain() -> TerrainConfig {
    if let Ok(resolved) = trident::target::ResolvedTarget::resolve("nox") {
        if matches!(resolved.vm.architecture, Arch::Tree) {
            return resolved.vm;
        }
    }
    // Mirror of trident/vm/nox/target.toml (2026-09-07).
    TerrainConfig {
        name: "nox".to_string(),
        display_name: "NOX".to_string(),
        architecture: Arch::Tree,
        field_prime: "2^64 - 2^32 + 1".to_string(),
        field_bits: 64,
        field_limbs: 2,
        emulated_fields: Vec::new(),
        stack_depth: 0,
        spill_ram_base: 0,
        digest_width: 8,
        xfield_width: 3,
        hash_rate: 8,
        output_extension: ".nox".to_string(),
        cost_tables: vec!["reductions".to_string()],
        warrior: Some(WarriorConfig {
            name: "joy".to_string(),
            crate_name: "joy".to_string(),
            runner: true,
            prover: false,
        }),
    }
}
