use std::path::Path;

use trident::runtime::ProgramBundle;
use trident::{compile_to_bundle, CompileOptions};

use crate::error::JoyError;

/// Compile a .tri source (or project dir) to a nox ProgramBundle
/// via the trident API.
pub fn compile_source(input: &Path, profile: &str) -> Result<ProgramBundle, JoyError> {
    let package = joy_rs::target_package("nox").map_err(JoyError::Compile)?;
    let options = CompileOptions::for_profile(profile)
        .with_package(package)
        .map_err(JoyError::Compile)?;
    let (entry, options) = trident::source_options(input, &options).map_err(|diagnostics| {
        JoyError::Compile(
            diagnostics
                .into_iter()
                .map(|d| d.message)
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    compile_to_bundle(&entry, &options).map_err(|diagnostics| {
        let messages: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
        JoyError::Compile(messages.join("; "))
    })
}
