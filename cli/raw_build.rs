//! Source packages to complete native ART1, published only after full encoding.
use crate::{build_cmd::BuildArgs, compile, error::JoyError, publication::atomic_write};
use std::path::PathBuf;

pub fn build(args: &BuildArgs) -> Result<(PathBuf, serde_json::Value), JoyError> {
    let resolved = compile::resolve_source(&args.input, &args.profile, args.target.as_deref())?;
    let profile = args
        .artifact_profile
        .unwrap_or(crate::build_cmd::ArtifactProfile::Raw)
        .native();
    let artifact = trident::compile_native_artifact_project(
        &resolved.entry,
        &resolved.options,
        profile,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .map_err(compile::diagnostics)?;
    let name = if args.input.is_dir() {
        resolved
            .project
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or(&artifact.name)
    } else {
        &artifact.name
    };
    let output = args.output.clone().unwrap_or_else(|| {
        if args.input.is_dir() {
            args.input.join(format!("{name}.dag"))
        } else {
            args.input.with_extension("dag")
        }
    });
    atomic_write(&output, &artifact.bytes, args.force)?;
    let particle: String = artifact
        .particle
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (owner, version, package_hash, compiler_api) = resolved.metadata;
    let result = serde_json::json!({
        "kind":"build", "artifact":output.to_string_lossy(), "format":"artifact",
        "program":name, "program_particle":particle, "artifact_bytes":artifact.bytes.len(),
        "target":resolved.target, "target_vm":"nox", "target_os":null,
        "profile":args.profile, "reads_state":false, "entry_point":"main",
        "input_profile":artifact.profile.value(), "output_profile":artifact.profile.value(),
        "compiler":{"name":"trident","version":trident::COMPILER_VERSION,"api":compiler_api},
        "target_package":{"owner":owner,"version":version,"compilation_hash":package_hash}
    });
    Ok((output, result))
}
