use super::{
    job_limits::JobLimits,
    particle,
    reader::{self, Reader, Result},
    RunLimits,
};
use nox::{artifact, Digest, Order, Reduction};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct ModuleReport {
    pub logical_path: String,
    pub origin_name: String,
    pub origin_version: String,
    pub particle: String,
    pub source_particle: String,
    pub source_bytes: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub target: u64,
    pub input_profile: u64,
    pub output_profile: u64,
    pub optimization: u64,
    pub cfg_flags: Vec<String>,
}

pub(super) struct Job {
    pub identity: Digest,
    pub package_particle: String,
    pub modules: Vec<ModuleReport>,
    pub entry_index: u32,
    pub entry_module: String,
    pub entry_function: String,
    pub options: Options,
    pub limits: JobLimits,
    pub input_visits: u32,
}

pub(super) fn program<const N: usize>(r: &mut Reader<'_, N>, root: Order) -> Result<(Order, u64)> {
    let fields = r.record::<4>(root, 0x41525431)?;
    let machine = r.field(fields[0])?;
    let input = r.field(fields[1])?;
    let output = r.field(fields[2])?;
    if machine != 0 || input > 1 || input != output {
        return Err("ART1 requires machine0 and matching supported profiles".into());
    }
    Ok((fields[3], input))
}

pub(super) fn check_container<const N: usize>(
    ar: &Reduction<N>,
    root: Order,
    limits: JobLimits,
) -> Result<()> {
    artifact::encode(ar, root, limits.transport())
        .map(|_| ())
        .map_err(|e| format!("requested artifact limits: {e:?}"))
}

pub(super) fn admit<const N: usize>(
    ar: &Reduction<N>,
    root: Order,
    compiler: Order,
    host: RunLimits,
    deadline: Instant,
    program_visits: u32,
) -> Result<Job> {
    let mut r = Reader {
        ar,
        remaining: host
            .compiler
            .validation_visits
            .checked_sub(program_visits)
            .ok_or("program validation visits")?,
        sequence_limit: host.compiler.sequence_length,
        deadline,
    };
    let fields = r.record::<6>(root, 0x4a4f4231)?;
    let expected = r.digest(fields[0])?;
    if ar.digest(compiler) != Some(&expected) {
        return Err("compiler identity mismatch".into());
    }
    let limit_nodes = r.record::<11>(fields[5], 0x4c494d31)?;
    let mut values = [0; 11];
    for (i, node) in limit_nodes.into_iter().enumerate() {
        values[i] = r.field(node)?;
    }
    let limits = JobLimits::admit(values, host)?;
    let spent = host.compiler.validation_visits - r.remaining;
    r.remaining = limits
        .validation_visits
        .checked_sub(spent)
        .ok_or("job validation visits exhausted")?;
    r.sequence_limit = limits.sequence_length;
    check_container(ar, compiler, limits).map_err(|e| format!("compiler container: {e}"))?;
    check_container(ar, root, limits).map_err(|e| format!("job container: {e}"))?;
    let entry_module = r.string(fields[2], 255)?;
    let entry_function = r.string(fields[3], 255)?;
    if !reader::identifier(&entry_module) || !reader::identifier(&entry_function) {
        return Err("entry identifier".into());
    }
    let opts = r.record::<5>(fields[4], 0x4f505431)?;
    let target = r.field(opts[0])?;
    let input_profile = r.field(opts[1])?;
    let output_profile = r.field(opts[2])?;
    let optimization = r.field(opts[3])?;
    if target != 0 || input_profile > 1 || input_profile != output_profile || optimization != 0 {
        return Err("unsupported compile option".into());
    }
    let mut cfg_flags = Vec::new();
    for node in r.list(opts[4], limits.sequence_length)? {
        let flag = r.string(node, 255)?;
        if !reader::identifier(&flag) || cfg_flags.last().is_some_and(|last| last >= &flag) {
            return Err("cfg order/name".into());
        }
        cfg_flags.push(flag);
    }
    let package = r.record::<1>(fields[1], 0x504b4731)?;
    let mut modules: Vec<ModuleReport> = Vec::new();
    let mut source_remaining = limits.source_bytes;
    for node in r.list(package[0], limits.modules)? {
        let m = r.record::<4>(node, 0x4d4f4431)?;
        let logical_path = r.string(m[0], 255)?;
        if !reader::module_path(&logical_path)
            || modules
                .last()
                .is_some_and(|last| last.logical_path >= logical_path)
        {
            return Err("module order/name".into());
        }
        let origin_name = r.string(m[1], 255)?;
        let origin_version = r.string(m[2], 255)?;
        if !reader::label(&origin_name) || !reader::label(&origin_version) {
            return Err("origin label".into());
        }
        // Validate exact bytes; only length/identity survive into result admission.
        let source_bytes = r.bytes(m[3], source_remaining)?.len() as u32;
        source_remaining -= source_bytes;
        modules.push(ModuleReport {
            logical_path,
            origin_name,
            origin_version,
            source_bytes,
            particle: particle(ar, node)?,
            source_particle: particle(ar, m[3])?,
        });
    }
    let entry_index = modules
        .iter()
        .position(|m| m.logical_path == entry_module)
        .ok_or("entry module absent")? as u32;
    Ok(Job {
        identity: *ar.digest(root).ok_or("missing job identity")?,
        package_particle: particle(ar, fields[1])?,
        modules,
        entry_index,
        entry_module,
        entry_function,
        options: Options {
            target,
            input_profile,
            output_profile,
            optimization,
            cfg_flags,
        },
        limits,
        input_visits: limits.validation_visits - r.remaining,
    })
}
