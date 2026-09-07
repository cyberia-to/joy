use std::path::Path;

use trident::runtime::ProgramBundle;
use trident::{compile_to_bundle, CompileOptions};

use crate::error::JoyError;

/// Compile a .tri source (or project dir) to a nox ProgramBundle
/// via the trident API.
pub fn compile_source(input: &Path, profile: &str) -> Result<ProgramBundle, JoyError> {
    let mut options = CompileOptions::for_profile(profile);
    options.target_config = joy_rs::nox_terrain();

    compile_to_bundle(input, &options).map_err(|diagnostics| {
        let messages: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
        JoyError::Compile(messages.join("; "))
    })
}
