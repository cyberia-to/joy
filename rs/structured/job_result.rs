use super::{
    job::{self, Job, ModuleReport, Options},
    job_limits::JobLimits,
    particle,
    reader::{Reader, Result},
};
use nox::{artifact, Order, Reduction};
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    // Canonical ordering differs from DIA1 wire order.
    pub module_index: u32,
    pub start_byte: u32,
    pub end_byte: u32,
    pub code: u32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CompilerReport {
    pub package_particle: String,
    pub modules: Vec<ModuleReport>,
    pub entry_module: String,
    pub entry_function: String,
    pub options: Options,
    pub limits: JobLimits,
    pub input_validation_visits: u32,
    pub output_validation_visits: u32,
    pub status: &'static str,
    pub diagnostics: Vec<Diagnostic>,
    pub compiled_particle: Option<String>,
}

pub(super) fn validate<const N: usize>(
    ar: &Reduction<N>,
    root: Order,
    job: Job,
    deadline: Instant,
) -> Result<(CompilerReport, Option<Vec<u8>>)> {
    job::check_container(ar, root, job.limits)?;
    let mut r = Reader {
        ar,
        remaining: job.limits.validation_visits,
        sequence_limit: job.limits.sequence_length,
        deadline,
    };
    let fields = r.record::<3>(root, 0x52455331)?;
    if r.digest(fields[0])? != job.identity {
        return Err("job identity mismatch".into());
    }
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let (status, compiled_particle, compiled) = match r.field(fields[1])? {
        0 => {
            if job::program(&mut r, fields[2])?.1 != job.options.input_profile {
                return Err("generated artifact profile mismatch".into());
            }
            let bytes = artifact::encode(ar, fields[2], job.limits.transport())
                .map_err(|e| format!("generated artifact: {e:?}"))?;
            ("success", Some(particle(ar, fields[2])?), Some(bytes))
        }
        1 => {
            let mut overflow = false;
            for node in r.list(fields[2], job.limits.diagnostics)? {
                let d = r.record::<5>(node, 0x44494131)?;
                let code = r.word(d[0])?;
                let module_index = r.word(d[1])?;
                let start_byte = r.word(d[2])?;
                let end_byte = r.word(d[3])?;
                let module = job
                    .modules
                    .get(module_index as usize)
                    .ok_or("diagnostic module")?;
                if !(1..=8).contains(&code)
                    || start_byte > end_byte
                    || end_byte > module.source_bytes
                {
                    return Err("diagnostic span/code".into());
                }
                if code == 8 {
                    if overflow
                        || module_index != job.entry_index
                        || start_byte != 0
                        || end_byte != 0
                    {
                        return Err("diagnostic limit marker".into());
                    }
                    overflow = true;
                }
                let message = r.string(d[4], job.limits.artifact_bytes)?;
                let diagnostic = Diagnostic {
                    module_index,
                    start_byte,
                    end_byte,
                    code,
                    message,
                };
                if diagnostics.last().is_some_and(|last| last > &diagnostic) {
                    return Err("diagnostic order".into());
                }
                diagnostics.push(diagnostic);
            }
            if diagnostics.is_empty() {
                return Err("empty compile-error result".into());
            }
            ("compile_error", None, None)
        }
        _ => return Err("result status".into()),
    };
    let report = CompilerReport {
        package_particle: job.package_particle,
        modules: job.modules,
        entry_module: job.entry_module,
        entry_function: job.entry_function,
        options: job.options,
        limits: job.limits,
        input_validation_visits: job.input_visits,
        output_validation_visits: job.limits.validation_visits - r.remaining,
        status,
        diagnostics,
        compiled_particle,
    };
    Ok((report, compiled))
}
