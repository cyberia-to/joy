use super::{
    job,
    pack_writer::Writer,
    particle,
    reader::{self, Reader, Result},
    JobLimits, ModuleReport, Options, RunLimits, ARENA, STACK,
};
use nox::{artifact, Order, Reduction};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleFile {
    logical_path: String,
    file: PathBuf,
    origin_name: String,
    origin_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    entry_module: String,
    entry_function: String,
    modules: Vec<ModuleFile>,
    options: Options,
    limits: JobLimits,
}

#[derive(Debug, Serialize)]
pub struct PackReport {
    pub compiler_particle: String,
    pub job_particle: String,
    pub package_particle: String,
    pub modules: Vec<ModuleReport>,
    pub entry_module: String,
    pub entry_function: String,
    pub options: Options,
    pub limits: JobLimits,
    pub validation_visits: u32,
}

#[derive(Debug)]
pub struct PackedJob {
    pub bytes: Vec<u8>,
    pub report: PackReport,
}

fn normalize(manifest: &mut Manifest, host: RunLimits) -> Result<()> {
    if manifest.version != 1 {
        return Err("unsupported package manifest version".into());
    }
    JobLimits::admit(manifest.limits.values(), host)?;
    if manifest.modules.len()
        > manifest.limits.modules.min(manifest.limits.sequence_length) as usize
    {
        return Err("package module count limit".into());
    }
    if !reader::identifier(&manifest.entry_module) || !reader::identifier(&manifest.entry_function)
    {
        return Err("package entry identifier".into());
    }
    manifest
        .modules
        .sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    let mut previous = None;
    for module in &manifest.modules {
        if !reader::module_path(&module.logical_path) || previous == Some(&module.logical_path) {
            return Err("package module name/duplicate".into());
        }
        if !reader::label(&module.origin_name) || !reader::label(&module.origin_version) {
            return Err("package origin label".into());
        }
        previous = Some(&module.logical_path);
    }
    if !manifest
        .modules
        .iter()
        .any(|m| m.logical_path == manifest.entry_module)
    {
        return Err("package entry module absent".into());
    }
    let options = &mut manifest.options;
    if options.target != 0
        || options.input_profile > 1
        || options.input_profile != options.output_profile
        || options.optimization != 0
    {
        return Err("unsupported package option".into());
    }
    if options.cfg_flags.len() > manifest.limits.sequence_length as usize {
        return Err("cfg count limit".into());
    }
    options.cfg_flags.sort();
    for (i, flag) in options.cfg_flags.iter().enumerate() {
        if !reader::identifier(flag) || (i > 0 && &options.cfg_flags[i - 1] == flag) {
            return Err("cfg name/duplicate".into());
        }
    }
    Ok(())
}

fn encode_job(
    w: &mut Writer<'_, ARENA>,
    compiler: Order,
    manifest: &Manifest,
    directory: &Path,
) -> Result<Order> {
    let mut remaining_source = manifest.limits.source_bytes as usize;
    let mut modules = Vec::new();
    for module in &manifest.modules {
        let path = if module.file.is_absolute() {
            module.file.clone()
        } else {
            directory.join(&module.file)
        };
        let bytes = crate::file_input::read(&path, remaining_source)
            .map_err(|e| format!("module {}: {e}", module.logical_path))?;
        remaining_source -= bytes.len();
        let path = w.bytes(module.logical_path.as_bytes())?;
        let origin = w.bytes(module.origin_name.as_bytes())?;
        let version = w.bytes(module.origin_version.as_bytes())?;
        let source = w.bytes(&bytes)?;
        modules.push(w.record(0x4d4f4431, &[path, origin, version, source])?);
    }
    let modules = w.seq(&modules)?;
    let package = w.record(0x504b4731, &[modules])?;
    let digest = *w.ar.digest(compiler).ok_or("compiler identity missing")?;
    let compiler = w.ar.hash_data(&digest).ok_or("package arena exhausted")?;
    let entry = w.bytes(manifest.entry_module.as_bytes())?;
    let function = w.bytes(manifest.entry_function.as_bytes())?;
    let opts = &manifest.options;
    let mut fields = Vec::new();
    for value in [
        opts.target,
        opts.input_profile,
        opts.output_profile,
        opts.optimization,
    ] {
        fields.push(w.atom(value)?);
    }
    let mut flags = Vec::new();
    for flag in &opts.cfg_flags {
        flags.push(w.bytes(flag.as_bytes())?);
    }
    fields.push(w.seq(&flags)?);
    let options = w.record(0x4f505431, &fields)?;
    let mut fields = Vec::new();
    for value in manifest.limits.values() {
        fields.push(w.atom(value)?);
    }
    let limits = w.record(0x4c494d31, &fields)?;
    w.record(
        0x4a4f4231,
        &[compiler, package, entry, function, options, limits],
    )
}

fn construct(
    compiler: Vec<u8>,
    mut manifest: Manifest,
    directory: PathBuf,
    host: RunLimits,
) -> Result<PackedJob> {
    let deadline = Instant::now() + Duration::from_millis(host.time_ms);
    normalize(&mut manifest, host)?;
    let mut ar = Reduction::<ARENA>::new();
    if !ar.limit_allocations(manifest.limits.arena_nodes) {
        return Err("package arena allowance".into());
    }
    let compiler = artifact::decode(&mut ar, &compiler, manifest.limits.transport())
        .map_err(|e| format!("compiler artifact: {e:?}"))?;
    let spent = {
        let mut r = Reader {
            ar: &ar,
            remaining: host.compiler.validation_visits,
            sequence_limit: host.compiler.sequence_length,
            deadline,
        };
        if job::program(&mut r, compiler)?.1 != 1 {
            return Err("pack-job requires compiler profile(1,1)".into());
        }
        host.compiler.validation_visits - r.remaining
    };
    let root = encode_job(
        &mut Writer {
            ar: &mut ar,
            deadline,
        },
        compiler,
        &manifest,
        &directory,
    )?;
    let job = job::admit(&ar, root, compiler, host, deadline, spent)?;
    let bytes = artifact::encode(&ar, root, manifest.limits.transport())
        .map_err(|e| format!("job artifact: {e:?}"))?;
    if Instant::now() >= deadline {
        return Err("package construction deadline exceeded".into());
    }
    Ok(PackedJob {
        bytes,
        report: PackReport {
            compiler_particle: particle(&ar, compiler)?,
            job_particle: particle(&ar, root)?,
            package_particle: job.package_particle,
            modules: job.modules,
            entry_module: job.entry_module,
            entry_function: job.entry_function,
            options: job.options,
            limits: job.limits,
            validation_visits: job.input_visits,
        },
    })
}

/// Serialize explicit files only. Source language work belongs to the guest.
pub fn pack_job_files(compiler: &Path, manifest: &Path, host: RunLimits) -> Result<PackedJob> {
    host.validate()?;
    let compiler = crate::file_input::read(compiler, host.artifact_bytes)?;
    let bytes = crate::file_input::read(manifest, host.artifact_bytes)?;
    let request: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("package manifest: {e}"))?;
    let directory = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();
    std::thread::Builder::new()
        .name("joy-job-pack".into())
        .stack_size(STACK)
        .spawn(move || construct(compiler, request, directory, host))
        .map_err(|e| format!("package worker spawn: {e}"))?
        .join()
        .map_err(|_| "package worker panicked".to_string())?
}
