use super::*;

#[test]
fn physical_arena_capacity_preserves_output_identity_and_execution_accounting() {
    let (program, input) = fixture(|ar| {
        let zero = atom(ar, 0);
        let value = quote(ar, 7);
        (binary(ar, 5, value, value), zero)
    });
    let small = run(program.clone(), input.clone(), RunLimits::default()).unwrap();
    assert_eq!(output_atom(&small.output), 14);
    assert_eq!(
        small.report.arena_reserved_bytes,
        std::mem::size_of::<Reduction<ARENA>>()
    );
    for (arena_nodes, reserved_bytes) in [
        (
            DEFAULT_ARENA_NODES + 1,
            std::mem::size_of::<Reduction<LARGE_ARENA>>(),
        ),
        (
            LARGE_ARENA_NODES,
            std::mem::size_of::<Reduction<LARGE_ARENA>>(),
        ),
        (
            LARGE_ARENA_NODES + 1,
            std::mem::size_of::<Reduction<COMPILER_ARENA>>(),
        ),
        (
            MAX_ARENA_NODES,
            std::mem::size_of::<Reduction<COMPILER_ARENA>>(),
        ),
    ] {
        let large = run(
            program.clone(),
            input.clone(),
            RunLimits {
                arena_nodes,
                ..RunLimits::default()
            },
        )
        .unwrap();
        assert_eq!(large.output, small.output);
        assert_eq!(large.compiled, small.compiled);
        assert_eq!(large.report.program_particle, small.report.program_particle);
        assert_eq!(large.report.input_particle, small.report.input_particle);
        assert_eq!(large.report.output_particle, small.report.output_particle);
        assert_eq!(
            large.report.charged_reductions,
            small.report.charged_reductions
        );
        assert_eq!(large.report.allocated_nodes, small.report.allocated_nodes);
        assert_eq!(large.report.peak_frames, small.report.peak_frames);
        assert_eq!(large.report.arena_reserved_bytes, reserved_bytes);
        assert_eq!(
            large.report.worker_stack_bytes,
            small.report.worker_stack_bytes
        );
    }
    assert_eq!(RunLimits::default().arena_nodes, 196608);
}

#[test]
fn compiler_scale_allowances_are_explicit_and_bounded() {
    let ordinary = RunLimits::default();
    assert_eq!(ordinary.budget, 1_000_000);
    assert_eq!(ordinary.time_ms, 30_000);
    let maximum = RunLimits {
        arena_nodes: MAX_ARENA_NODES,
        budget: MAX_BUDGET,
        time_ms: MAX_TIME_MS,
        ..ordinary
    };
    maximum.validate().unwrap();
    for invalid in [
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
    ] {
        assert!(invalid.validate().is_err());
    }
}
