use crate::error::JoyError;
use std::path::Path;
use trident::{compile_to_bundle, project::Project, runtime::ProgramBundle, CompileOptions};

pub fn project(input: &Path) -> Result<Option<Project>, JoyError> {
    let manifest = if input.is_dir() {
        Some(input.join("trident.toml"))
    } else {
        Project::find(input.parent().unwrap_or(Path::new(".")))
    };
    manifest
        .map(|path| Project::load(&path).map_err(|e| JoyError::Compile(e.message)))
        .transpose()
}
/// One resolution path for build, run and prove; explicit > project > nox.
pub fn compile_source(
    input: &Path,
    profile: &str,
    target: Option<&str>,
) -> Result<ProgramBundle, JoyError> {
    Ok(compile_source_with_metadata(input, profile, target)?.bundle)
}

pub struct CompiledSource {
    pub bundle: ProgramBundle,
    pub target: String,
    pub package_owner: String,
    pub package_version: String,
    pub package_hash: String,
    pub compiler_api: u32,
}

pub fn compile_source_with_metadata(
    input: &Path,
    profile: &str,
    target: Option<&str>,
) -> Result<CompiledSource, JoyError> {
    let resolved = resolve_source(input, profile, target)?;
    let ResolvedSource {
        project,
        target,
        metadata,
        entry,
        options,
    } = resolved;
    let mut bundle = compile_to_bundle(&entry, &options).map_err(diagnostics)?;
    if input.is_dir() {
        if let Some(project) = &project {
            bundle.name = project.name.clone();
            bundle.version = project.version.clone();
        }
    }
    Ok(CompiledSource {
        bundle,
        target,
        package_owner: metadata.0,
        package_version: metadata.1,
        package_hash: metadata.2,
        compiler_api: metadata.3,
    })
}

/// Shared resolution also feeds structured ART1 emission.
pub struct ResolvedSource {
    pub project: Option<Project>,
    pub target: String,
    pub metadata: (String, String, String, u32),
    pub entry: std::path::PathBuf,
    pub options: CompileOptions,
}

pub fn diagnostics(errors: Vec<trident::Diagnostic>) -> JoyError {
    JoyError::Compile(
        errors
            .into_iter()
            .map(|d| d.message)
            .collect::<Vec<_>>()
            .join("; "),
    )
}

pub fn resolve_source(
    input: &Path,
    profile: &str,
    target: Option<&str>,
) -> Result<ResolvedSource, JoyError> {
    let project = project(input)?;
    let target = target
        .or_else(|| project.as_ref().and_then(|p| p.target.as_deref()))
        .unwrap_or("nox");
    crate::check_target(target)?;
    if !matches!(profile, "debug" | "release")
        && !project
            .as_ref()
            .is_some_and(|p| p.targets.contains_key(profile))
    {
        return Err(JoyError::Compile(format!(
            "unknown compilation profile '{profile}'"
        )));
    }
    let package = joy_rs::target_package(target).map_err(JoyError::Compile)?;
    let metadata = (
        package.owner.clone(),
        package.version.clone(),
        package.compilation_hash().map_err(JoyError::Compile)?,
        package.compiler_api,
    );
    let options = CompileOptions::for_profile(profile)
        .with_package(package)
        .map_err(JoyError::Compile)?;
    let (entry, options) = trident::source_options(input, &options).map_err(diagnostics)?;
    let target = target.to_string();
    Ok(ResolvedSource {
        project,
        target,
        metadata,
        entry,
        options,
    })
}
