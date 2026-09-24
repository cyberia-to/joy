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
    for arena_nodes in [DEFAULT_ARENA_NODES + 1, MAX_ARENA_NODES] {
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
        assert_eq!(
            large.report.arena_reserved_bytes,
            std::mem::size_of::<Reduction<LARGE_ARENA>>()
        );
        assert_eq!(
            large.report.worker_stack_bytes,
            small.report.worker_stack_bytes
        );
    }
    assert_eq!(RunLimits::default().arena_nodes, 196608);
}
