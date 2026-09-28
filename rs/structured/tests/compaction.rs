use super::*;

fn limits() -> RunLimits {
    RunLimits {
        compaction: Some(CompactionPolicy {
            resident_nodes: 256,
            collection_work: 100_000_000,
        }),
        ..RunLimits::default()
    }
}

#[test]
fn reclaiming_worker_preserves_nouns_and_gas_beyond_the_resident_allowance() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 2000, false));
    let ordinary = run(program.clone(), input.clone(), RunLimits::default()).unwrap();
    let tight = RunLimits {
        arena_nodes: 256,
        ..RunLimits::default()
    };
    assert!(run(program.clone(), input.clone(), tight)
        .unwrap_err()
        .contains("Unavailable"));
    let compacted = run(program.clone(), input.clone(), limits()).unwrap();
    assert_eq!(compacted.output, ordinary.output);
    assert_eq!(
        compacted.report.program_particle,
        ordinary.report.program_particle
    );
    assert_eq!(
        compacted.report.input_particle,
        ordinary.report.input_particle
    );
    assert_eq!(
        compacted.report.output_particle,
        ordinary.report.output_particle
    );
    assert_eq!(
        compacted.report.charged_reductions,
        ordinary.report.charged_reductions
    );
    assert_eq!(compacted.report.peak_frames, ordinary.report.peak_frames);
    assert_eq!(output_atom(&compacted.output), 2000);
    assert!(ordinary.report.compaction.is_none());
    let stats = compacted.report.compaction.unwrap();
    assert!(stats.collections > 1);
    assert!(stats.reclaimed_nodes > 0);
    assert!(stats.resident_nodes <= stats.peak_resident_nodes);
    assert!(stats.peak_resident_nodes <= 256);
    assert!(stats.total_allocations > 256);
    assert_eq!(
        stats.total_allocations,
        u64::from(compacted.report.allocated_nodes)
    );
    assert_eq!(
        stats.total_allocations,
        stats.reclaimed_nodes + u64::from(stats.resident_nodes)
    );
    assert_eq!(
        stats.scratch_bytes,
        sequential::compaction_storage_bytes(ARENA, 256).unwrap()
    );
    assert_eq!(
        compacted.report.arena_reserved_bytes,
        std::mem::size_of::<Reduction<ARENA>>()
    );
}

#[test]
fn collection_never_replenishes_cumulative_or_work_allowances() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 100, false));
    let run_limits = limits();
    let measured = run(program.clone(), input.clone(), run_limits).unwrap();
    let stats = measured.report.compaction.unwrap();
    let exact = RunLimits {
        arena_nodes: measured.report.allocated_nodes,
        compaction: Some(CompactionPolicy {
            collection_work: stats.collection_work,
            ..run_limits.compaction.unwrap()
        }),
        ..run_limits
    };
    assert_eq!(
        run(program.clone(), input.clone(), exact).unwrap().output,
        measured.output
    );
    let total = RunLimits {
        arena_nodes: exact.arena_nodes - 1,
        ..exact
    };
    assert!(run(program.clone(), input.clone(), total)
        .unwrap_err()
        .contains("TotalAllocations"));
    let work = RunLimits {
        compaction: Some(CompactionPolicy {
            collection_work: stats.collection_work - 1,
            ..exact.compaction.unwrap()
        }),
        ..exact
    };
    assert!(run(program.clone(), input.clone(), work)
        .unwrap_err()
        .contains("CollectionWork"));
    let loading = RunLimits {
        compaction: Some(CompactionPolicy {
            resident_nodes: stats.pinned_nodes - 1,
            ..exact.compaction.unwrap()
        }),
        ..exact
    };
    let error = run(program, input, loading).unwrap_err();
    assert!(error.contains("artifact:"), "{error}");
}

#[test]
fn compacting_policy_keeps_resource_and_service_rejections() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 100, false));
    for (limits, message) in [
        (
            RunLimits {
                budget: 10,
                ..limits()
            },
            "budget exhausted",
        ),
        (
            RunLimits {
                frames: 1,
                ..limits()
            },
            "Frames",
        ),
    ] {
        assert!(run(program.clone(), input.clone(), limits)
            .unwrap_err()
            .contains(message));
    }
    for tag in [16, 17] {
        let (program, input) = fixture(|ar| {
            let zero = atom(ar, 0);
            let request = quote(ar, 0);
            (op(ar, tag, request), zero)
        });
        let error = run(program, input, limits()).unwrap_err();
        assert!(
            error.contains(&format!("UnsupportedService({tag})")),
            "{error}"
        );
    }
}

#[test]
fn extended_caps_require_explicit_compaction_and_remain_bounded() {
    let maximum = RunLimits {
        arena_nodes: 1_000_000_000,
        budget: 20_000_000_000,
        time_ms: 7_200_000,
        compaction: Some(CompactionPolicy {
            resident_nodes: MAX_ARENA_NODES,
            collection_work: 10_000_000_000,
        }),
        ..RunLimits::default()
    };
    maximum.validate().unwrap();
    RunLimits {
        budget: 10_000_000_000,
        ..maximum
    }
    .validate()
    .unwrap();
    assert_eq!(maximum.resident_nodes(), MAX_ARENA_NODES);
    assert_eq!(limits().resident_nodes(), 256);
    for invalid in [
        RunLimits {
            compaction: None,
            ..maximum
        },
        RunLimits {
            arena_nodes: maximum.arena_nodes + 1,
            ..maximum
        },
        RunLimits {
            budget: maximum.budget + 1,
            ..maximum
        },
        RunLimits {
            time_ms: maximum.time_ms + 1,
            ..maximum
        },
        RunLimits {
            compaction: Some(CompactionPolicy {
                resident_nodes: 0,
                ..maximum.compaction.unwrap()
            }),
            ..maximum
        },
        RunLimits {
            compaction: Some(CompactionPolicy {
                resident_nodes: MAX_ARENA_NODES + 1,
                ..maximum.compaction.unwrap()
            }),
            ..maximum
        },
        RunLimits {
            compaction: Some(CompactionPolicy {
                collection_work: 0,
                ..maximum.compaction.unwrap()
            }),
            ..maximum
        },
        RunLimits {
            compaction: Some(CompactionPolicy {
                collection_work: 10_000_000_001,
                ..maximum.compaction.unwrap()
            }),
            ..maximum
        },
    ] {
        assert!(invalid.validate().is_err(), "{invalid:?}");
    }
}

#[test]
fn explicit_compaction_deadlines_preserve_defaults_and_cancellation() {
    let defaults = RunLimits::default();
    assert_eq!(defaults.time_ms, 30_000);
    assert_eq!(MAX_TIME_MS, 300_000);
    RunLimits {
        time_ms: MAX_TIME_MS,
        ..defaults
    }
    .validate()
    .unwrap();
    assert!(RunLimits {
        time_ms: MAX_TIME_MS + 1,
        ..defaults
    }
    .validate()
    .is_err());

    for time_ms in [3_600_000, 7_200_000] {
        RunLimits {
            time_ms,
            ..limits()
        }
        .validate()
        .unwrap();
    }

    // A short expired worker start exercises evaluator cancellation without
    // requiring a clock that can represent times hours before process startup.
    let run_limits = RunLimits {
        time_ms: 1,
        ..limits()
    };
    let mut ar = Arena::new();
    let (formula, input) = loop_fixture(&mut ar, 2000, false);
    assert!(ar.limit_allocations(run_limits.resident_nodes()));
    let started = Instant::now() - Duration::from_millis(2);
    let error = match execution::run(
        &mut ar,
        execution::Request {
            input,
            formula,
            budget: run_limits.budget,
            allocations: run_limits.arena_nodes,
            frames: sequential::Limits {
                max_frames: run_limits.frames,
            },
        },
        run_limits,
        started,
    ) {
        Ok(_) => panic!("expired compacting execution produced a result"),
        Err(error) => error,
    };
    assert!(error.contains("Execution(Cancelled)"), "{error}");
}

#[test]
fn explicit_larger_reduction_allowance_preserves_prior_execution_and_defaults() {
    let defaults = RunLimits::default();
    assert_eq!(defaults.budget, 1_000_000);
    assert!(defaults.compaction.is_none());
    assert_eq!(MAX_BUDGET, 100_000_000);
    let ordinary_maximum = RunLimits {
        budget: MAX_BUDGET,
        ..defaults
    };
    ordinary_maximum.validate().unwrap();
    assert!(RunLimits {
        budget: MAX_BUDGET + 1,
        ..ordinary_maximum
    }
    .validate()
    .is_err());

    let (program, input) = fixture(|ar| loop_fixture(ar, 100, false));
    let prior = run(program.clone(), input.clone(), limits()).unwrap();
    for budget in [10_000_000_000, 20_000_000_000] {
        let result = run(
            program.clone(),
            input.clone(),
            RunLimits { budget, ..limits() },
        )
        .unwrap();
        assert_eq!(result.output, prior.output);
        assert_eq!(
            result.report.charged_reductions,
            prior.report.charged_reductions
        );
        assert_eq!(result.report.allocated_nodes, prior.report.allocated_nodes);
        assert_eq!(result.report.peak_frames, prior.report.peak_frames);
        let old_stats = prior.report.compaction.as_ref().unwrap();
        let stats = result.report.compaction.unwrap();
        assert_eq!(stats.total_allocations, old_stats.total_allocations);
        assert_eq!(stats.collection_work, old_stats.collection_work);
        assert_eq!(stats.collections, old_stats.collections);
    }
}
