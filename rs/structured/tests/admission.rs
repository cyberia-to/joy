use super::*;

#[test]
fn exact_art1_fields_and_terminator_are_enforced() {
    for case in 0..8 {
        let mut ar = Arena::new();
        let zero = atom(&mut ar, 0);
        let one = atom(&mut ar, 1);
        let formula = quote(&mut ar, 14);
        let mut fields = vec![zero, zero, zero, formula];
        match case {
            0 => fields[0] = one,
            1 => fields[1] = one,
            2 => fields[2] = one,
            3 => {
                fields.pop();
            }
            4 => fields.push(zero),
            5 => fields[0] = ar.pair(zero, zero).unwrap(),
            _ => (),
        }
        let mut tail = if case == 6 { one } else { zero };
        for field in fields.into_iter().rev() {
            tail = ar.pair(field, tail).unwrap();
        }
        let tag = if case == 7 { ART1 + 1 } else { ART1 };
        let p = op(&mut ar, tag, tail);
        assert!(
            run(encoded(&ar, p), encoded(&ar, zero), RunLimits::default())
                .unwrap_err()
                .contains("ART1")
        );
    }
}

#[test]
fn exact_input_bounds_and_shared_code_input_charge_once() {
    let (program, _) = fixture(|ar| {
        let zero = atom(ar, 0);
        (axis(ar, 1), zero)
    });
    let nodes = u32::from_le_bytes(program[40..44].try_into().unwrap());
    let limits = RunLimits {
        artifact_bytes: program.len(),
        artifact_nodes: nodes,
        arena_nodes: nodes,
        ..RunLimits::default()
    };
    let result = run(program.clone(), program.clone(), limits).unwrap();
    assert_eq!(result.output, program);
    assert_eq!(result.report.allocated_nodes, nodes);
    assert_eq!(result.report.program_particle, result.report.input_particle);
    assert!(run(
        program.clone(),
        program.clone(),
        RunLimits {
            arena_nodes: nodes - 1,
            ..limits
        }
    )
    .is_err());
    assert!(run(
        program.clone(),
        program.clone(),
        RunLimits {
            artifact_nodes: nodes - 1,
            ..limits
        }
    )
    .is_err());
    assert!(run(
        program.clone(),
        program.clone(),
        RunLimits {
            artifact_bytes: program.len() - 1,
            ..limits
        }
    )
    .is_err());
    let mut damaged = program.clone();
    damaged.pop();
    assert!(run(program.clone(), damaged, limits)
        .unwrap_err()
        .starts_with("input artifact:"));
    let mut oversized = program.clone();
    oversized.push(0);
    assert!(run(program, oversized, limits)
        .unwrap_err()
        .contains("size limit"));
}

#[test]
fn worker_deadline_returns_failure_without_output_bytes() {
    let (p, i) = fixture(|ar| loop_fixture(ar, 50_000, false));
    let error = run(
        p,
        i,
        RunLimits {
            time_ms: 1,
            ..RunLimits::default()
        },
    )
    .unwrap_err();
    assert!(
        error.contains("deadline") || error.contains("Cancelled"),
        "{error}"
    );
}
