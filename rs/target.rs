//! The nox machine contract is owned upstream and resolved by Trident.
//! Joy adds runtime capabilities, never a second set of machine constants.

use trident::target::TerrainConfig;

/// The same embedded nox machine contract used by the compiler.
pub fn nox_terrain() -> TerrainConfig {
    TerrainConfig::nox()
}

/// Versioned installed target package. Machine values come from Trident's
/// upstream nox contract; only warrior capabilities are owned by Joy.
pub fn target_package(target: &str) -> Result<trident::target::TargetPackage, String> {
    if !matches!(target, "nox" | "cyber") {
        return Err(format!("joy does not provide target '{target}'"));
    }
    let terrain = nox_terrain();
    let runtime = serde_json::from_str(include_str!("../targets/nox/capabilities.json"))
        .map_err(|error| format!("invalid embedded Joy capabilities: {error}"))?;
    trident::target::TargetPackage {
        schema_version: 1,
        compiler_api: 1,
        owner: "joy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        intrinsics: terrain.supported_intrinsics(),
        terrain,
        union: None,
        states: Vec::new(),
        modules: std::collections::BTreeMap::new(),
        module_hashes: std::collections::BTreeMap::new(),
        instructions: Vec::new(),
        runtime,
    }
    .seal()
}

/// JSON transport for `joy describe`, independent of the working directory.
pub fn describe(target: &str) -> Result<String, String> {
    serde_json::to_string_pretty(&target_package(target)?)
        .map_err(|error| format!("cannot encode Joy target package: {error}"))
}
