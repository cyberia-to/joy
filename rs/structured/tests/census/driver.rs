use super::*;
use nox::sequential::observe::{reduce_compacting_observed_controlled, CaptureLimits};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(super) fn execute<const N: usize>(
    program: &[u8],
    input: &[u8],
    limits: RunLimits,
    caps: Caps,
    capture: CaptureLimits,
) -> Result<serde_json::Value, String> {
    limits.validate()?;
    caps.validate()?;
    if limits.time_ms > 120_000
        || capture.max_events > 4_000_000
        || capture.max_bytes > 1 << 30
        || capture.max_work > 16_000_000
        || program.len() > limits.artifact_bytes
        || input.len() > limits.artifact_bytes
    {
        return Err("census hard ceiling".into());
    }
    let policy = limits
        .compaction
        .ok_or("census requires explicit compaction")?;
    let started = Instant::now();
    let expires = started + Duration::from_millis(limits.time_ms);
    let mut ar = Reduction::<N>::try_new_boxed().map_err(|e| format!("arena: {e}"))?;
    if !ar.limit_allocations(limits.resident_nodes()) {
        return Err("resident allowance".into());
    }
    let transport = limits.transport();
    let program =
        artifact::decode(&mut ar, program, transport).map_err(|e| format!("program: {e:?}"))?;
    let mut reader = reader::Reader {
        ar: &ar,
        remaining: limits.compiler.validation_visits,
        sequence_limit: limits.compiler.sequence_length,
        deadline: expires,
    };
    let (formula, profile) = job::program(&mut reader, program)?;
    let program_visits = limits.compiler.validation_visits - reader.remaining;
    let input = artifact::decode(&mut ar, input, transport).map_err(|e| format!("input: {e:?}"))?;
    let admitted = if profile == 1 {
        Some(job::admit(
            &ar,
            input,
            program,
            limits,
            expires,
            program_visits,
        )?)
    } else {
        None
    };
    let (budget, frames, allocations) =
        admitted
            .as_ref()
            .map_or((limits.budget, limits.frames, limits.arena_nodes), |j| {
                (
                    j.limits.reductions,
                    j.limits.evaluator_frames,
                    j.limits.arena_nodes,
                )
            });
    if !ar.limit_allocations(allocations.min(limits.resident_nodes())) {
        return Err("admitted allowance".into());
    }
    if frames as usize > caps.frames {
        return Err("census frames below admitted frames".into());
    }
    let identities = serde_json::json!({"program": particle(&ar, program)?,
        "formula": particle(&ar, formula)?, "subject": particle(&ar, input)?,
        "subject_is_unmodified_input": true});
    let admission = admitted.as_ref().map(|j| {
        serde_json::json!({"limits": j.limits,
        "module_count": j.modules.len(), "package_particle": j.package_particle,
        "entry_module": j.entry_module, "entry_function": j.entry_function,
        "input_validation_visits": j.input_visits})
    });
    let stop = Cell::new(false);
    let mut census = Census::new(caps, &stop)?;
    let setup_micros = started.elapsed().as_micros();
    let execution_started = Instant::now();
    let observed = reduce_compacting_observed_controlled(
        &mut ar,
        input,
        formula,
        budget,
        sequential::CompactionLimits {
            max_frames: frames,
            max_total_allocations: allocations as u64,
            max_collection_work: policy.collection_work,
        },
        capture,
        &mut census,
        &mut || stop.get() || Instant::now() >= expires,
    );
    let execution_micros = execution_started.elapsed().as_micros();
    let (stats, peak_frames, captured, failure, outcome) = match observed {
        Ok(run) => (
            run.execution.stats,
            run.execution.peak_frames,
            run.capture,
            None,
            Some(run.execution.outcome),
        ),
        Err(run) => (
            run.stats,
            run.peak_frames,
            run.capture,
            Some(format!("{:?}", run.kind)),
            None,
        ),
    };
    let success = match outcome {
        Some(nox::Outcome::Ok(value, remaining)) => {
            if census.counts.completed_events != 1
                || census.counts.transitions + 1 != stats.evaluator_checkpoints
            {
                return Err("completed census/checkpoint mismatch".into());
            }
            let result_validation = admitted
                .map(|job| job_result::validate(&ar, value, job, expires).map(|(report, _)| report))
                .transpose()?;
            Some(
                serde_json::json!({"result_particle": particle(&ar, value)?, "remaining_budget": remaining,
                "charged_reductions": budget.checked_sub(remaining).ok_or("remaining exceeds budget")?,
                "compiler_result": result_validation}),
            )
        }
        _ => None,
    };
    Ok(serde_json::json!({
        "schema": "joy/compiler-proof-census/v1", "scope": "host prefix observations; no proof or SH7/SH8 closure",
        "status": if success.is_some() { "completed" } else if stop.get() { "prefix-ceiling" } else { "failed" },
        "identities": identities, "admission": admission, "census": census.summary(),
        "failure": failure, "raw_outcome": outcome.map(|o| format!("{o:?}")), "success": success,
        "initial_budget": budget, "max_frames": frames, "peak_frames": peak_frames,
        "setup_micros": setup_micros, "execution_micros": execution_micros,
        "arena_reserved_bytes": std::mem::size_of::<Reduction<N>>(),
        "capture_caps": {"events": capture.max_events, "bytes": capture.max_bytes, "work": capture.max_work},
        "capture_attempts": {"events": captured.events, "bytes": captured.bytes, "work": captured.work},
        "physical": {"pinned_nodes": stats.pinned_nodes, "resident_nodes": stats.resident_nodes,
            "peak_resident_nodes": stats.peak_resident_nodes, "total_allocations": stats.total_allocations,
            "reclaimed_nodes": stats.reclaimed_nodes, "collections": stats.collections,
            "collection_work": stats.collection_work, "scratch_bytes": stats.scratch_bytes,
            "evaluator_checkpoints": stats.evaluator_checkpoints, "collection_checkpoints": stats.collection_checkpoints},
        "output_disk_cap_bytes": 1 << 20, "trace_disk_bytes": 0
    }))
}

#[test]
#[ignore = "bounded actual ART1/JOB1 diagnostic; requires JOY_CENSUS_PROGRAM, INPUT, OUTPUT"]
fn frozen_compiler_prefix() {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let program =
                PathBuf::from(std::env::var_os("JOY_CENSUS_PROGRAM").expect("compiler ART1"));
            let input = PathBuf::from(std::env::var_os("JOY_CENSUS_INPUT").expect("compiler JOB1"));
            let output =
                PathBuf::from(std::env::var_os("JOY_CENSUS_OUTPUT").expect("new output path"));
            assert!(!output.exists(), "refuse existing output");
            let transitions = std::env::var("JOY_CENSUS_TRANSITIONS")
                .unwrap_or_else(|_| "100000".into())
                .parse()
                .unwrap();
            let limits = RunLimits {
                budget: 20_000_000_000,
                arena_nodes: 1_000_000_000,
                frames: 65_536,
                time_ms: 120_000,
                compiler: CompilerCaps {
                    validation_visits: 16_777_216,
                    ..CompilerCaps::default()
                },
                compaction: Some(CompactionPolicy {
                    resident_nodes: 3_145_728,
                    collection_work: 10_000_000_000,
                }),
                ..RunLimits::default()
            };
            let program = crate::file_input::read(&program, limits.artifact_bytes).unwrap();
            let input = crate::file_input::read(&input, limits.artifact_bytes).unwrap();
            let report = execute::<{ 1 << 22 }>(
                &program,
                &input,
                limits,
                Caps {
                    transitions,
                    nouns: 262_144,
                    evaluations: 262_144,
                    frames: 65_536,
                },
                CaptureLimits {
                    max_events: 4_000_000,
                    max_bytes: 1 << 30,
                    max_work: 16_000_000,
                },
            )
            .unwrap();
            let bytes = serde_json::to_vec_pretty(&report).unwrap();
            assert!(bytes.len() < 1 << 20); // Include the trailing newline in the cap.
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)
                .unwrap();
            file.write_all(&bytes).unwrap();
            file.write_all(b"\n").unwrap();
            assert_eq!(report["status"], "prefix-ceiling", "{report}");
            assert_eq!(report["census"]["counts"]["transitions"], transitions);
            assert!(matches!(
                report["failure"].as_str(),
                Some("Execution(Execution(Cancelled))" | "Capture(Cancelled)")
            ));
        })
        .unwrap()
        .join()
        .unwrap();
}
