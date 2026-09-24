use super::*;
use nebu::Goldilocks;

mod admission;
mod compiler;

type Arena = Reduction<4096>;
fn atom(ar: &mut Arena, value: u64) -> Order {
    ar.atom(Goldilocks::new(value)).unwrap()
}
fn op(ar: &mut Arena, tag: u64, body: Order) -> Order {
    let t = atom(ar, tag);
    ar.pair(t, body).unwrap()
}
fn quote(ar: &mut Arena, value: u64) -> Order {
    let v = atom(ar, value);
    op(ar, 1, v)
}
fn axis(ar: &mut Arena, value: u64) -> Order {
    let v = atom(ar, value);
    op(ar, 0, v)
}
fn binary(ar: &mut Arena, tag: u64, a: Order, b: Order) -> Order {
    let p = ar.pair(a, b).unwrap();
    op(ar, tag, p)
}
fn program(ar: &mut Arena, formula: Order, profile: u64) -> Order {
    let zero = atom(ar, 0);
    let profile = atom(ar, profile);
    let mut rest = zero;
    for field in [zero, profile, profile, formula].into_iter().rev() {
        rest = ar.pair(field, rest).unwrap();
    }
    op(ar, ART1, rest)
}
fn encoded(ar: &Arena, root: Order) -> Vec<u8> {
    artifact::encode(ar, root, RunLimits::default().transport()).unwrap()
}
fn fixture(build: impl FnOnce(&mut Arena) -> (Order, Order)) -> (Vec<u8>, Vec<u8>) {
    let mut ar = Arena::new();
    let (formula, input) = build(&mut ar);
    let program = program(&mut ar, formula, 0);
    (encoded(&ar, program), encoded(&ar, input))
}
fn output_atom(bytes: &[u8]) -> u64 {
    let mut ar = Arena::new();
    let root = artifact::decode(&mut ar, bytes, RunLimits::default().transport()).unwrap();
    ar.atom_value(root).unwrap().as_u64()
}
fn loop_fixture(ar: &mut Arena, iterations: u64, build_chain: bool) -> (Order, Order) {
    let q0 = quote(ar, 0);
    let q1 = quote(ar, 1);
    let code = axis(ar, 2);
    let remaining = axis(ar, 6);
    let acc = axis(ar, 7);
    let done = binary(ar, 9, remaining, q0);
    let next = binary(ar, 6, remaining, q1);
    let acc_next = if build_chain {
        binary(ar, 3, remaining, acc)
    } else {
        binary(ar, 5, acc, q1)
    };
    let state = binary(ar, 3, next, acc_next);
    let subject = binary(ar, 3, code, state);
    let again = binary(ar, 2, subject, code);
    let arms = ar.pair(acc, again).unwrap();
    let formula = binary(ar, 4, done, arms);
    let zero = atom(ar, 0);
    let n = atom(ar, iterations);
    let state = ar.pair(n, zero).unwrap();
    (formula, ar.pair(formula, state).unwrap())
}

#[test]
fn raw_topology_and_shared_dag_roundtrip_without_flattening() {
    let mut particles = Vec::new();
    for shape in 0..4 {
        let (program, input) = fixture(|ar| {
            let a = atom(ar, 1);
            let b = atom(ar, 2);
            let c = atom(ar, 3);
            let root = match shape {
                0 => {
                    let p = ar.pair(a, b).unwrap();
                    ar.pair(p, c).unwrap()
                }
                1 => {
                    let p = ar.pair(b, c).unwrap();
                    ar.pair(a, p).unwrap()
                }
                2 => atom(ar, 0),
                _ => {
                    let mut p = a;
                    for _ in 0..100 {
                        p = ar.pair(p, p).unwrap();
                    }
                    p
                }
            };
            (axis(ar, 1), root)
        });
        let result = run(program, input.clone(), RunLimits::default()).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.report.input_particle, result.report.output_particle);
        assert_eq!(result.report.charged_reductions, 1);
        assert_eq!(result.report.trace_mode, "none");
        particles.push(result.report.output_particle);
        assert!(result.output.len() < 11_000);
    }
    assert_ne!(particles[0], particles[1]);
}

#[test]
fn runtime_generated_formula_executes_and_reports_bound_identities() {
    let (program, input) = fixture(|ar| {
        let zero = atom(ar, 0);
        let one = atom(ar, 1);
        let input = ar.pair(one, zero).unwrap();
        let a = quote(ar, 0);
        let tag = axis(ar, 2);
        let value = quote(ar, 42);
        let generated = binary(ar, 3, tag, value);
        (binary(ar, 2, a, generated), input)
    });
    let result = run(program.clone(), input.clone(), RunLimits::default()).unwrap();
    assert_eq!(output_atom(&result.output), 42);
    assert_eq!(result.report.charged_reductions, 6);
    for (bytes, particle) in [
        (&program, &result.report.program_particle),
        (&input, &result.report.input_particle),
        (&result.output, &result.report.output_particle),
    ] {
        assert_eq!(
            bytes[8..40]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            *particle
        );
    }
}

#[test]
fn resource_boundaries_apply_during_evaluation_not_only_loading() {
    let (program, input) = fixture(|ar| {
        let zero = atom(ar, 0);
        let q = quote(ar, 7);
        (binary(ar, 5, q, q), zero)
    });
    let limits = RunLimits {
        budget: 3,
        frames: 2,
        ..RunLimits::default()
    };
    let result = run(program.clone(), input.clone(), limits).unwrap();
    assert_eq!(output_atom(&result.output), 14);
    assert_eq!(result.report.charged_reductions, 3);
    assert_eq!(result.report.peak_frames, 2);
    let exact = RunLimits {
        arena_nodes: result.report.allocated_nodes,
        ..limits
    };
    run(program.clone(), input.clone(), exact).unwrap();
    for (bad, message) in [
        (
            RunLimits {
                arena_nodes: exact.arena_nodes - 1,
                ..limits
            },
            "Unavailable",
        ),
        (
            RunLimits {
                frames: 1,
                ..limits
            },
            "Frames",
        ),
        (
            RunLimits {
                budget: 2,
                ..limits
            },
            "budget exhausted",
        ),
    ] {
        assert!(run(program.clone(), input.clone(), bad)
            .unwrap_err()
            .contains(message));
    }
}

#[test]
fn output_transport_limits_are_checked_after_runtime_construction() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 200, true));
    let result = run(program.clone(), input.clone(), RunLimits::default()).unwrap();
    assert!(result.output.len() > program.len());
    for limits in [
        RunLimits {
            artifact_depth: 32,
            ..RunLimits::default()
        },
        RunLimits {
            artifact_nodes: 100,
            ..RunLimits::default()
        },
        RunLimits {
            artifact_bytes: program.len().max(input.len()),
            ..RunLimits::default()
        },
    ] {
        assert!(run(program.clone(), input.clone(), limits)
            .unwrap_err()
            .starts_with("output artifact:"));
    }
}

#[test]
fn compact_4097_iteration_program_runs_through_joy() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 4097, false));
    let result = run(program.clone(), input.clone(), RunLimits::default()).unwrap();
    assert_eq!(output_atom(&result.output), 4097);
    assert_eq!(result.report.charged_reductions, 61_460);
    assert!(program.len() < 5000);
    assert!(result.report.peak_frames > 4097);
    assert_eq!(
        result.report.frame_buffer_bytes,
        sequential::frame_storage_bytes(sequential::Limits { max_frames: 16_384 }).unwrap()
    );
    assert!(run(
        program,
        input,
        RunLimits {
            frames: result.report.peak_frames - 1,
            ..RunLimits::default()
        }
    )
    .unwrap_err()
    .contains("Frames"));
}

#[test]
fn malformed_containers_profiles_and_host_services_are_rejected() {
    let (program, input) = fixture(|ar| {
        let zero = atom(ar, 0);
        (quote(ar, 14), zero)
    });
    for n in [0, 1, 8, 39, program.len() - 1] {
        assert!(
            run(program[..n].to_vec(), input.clone(), RunLimits::default())
                .unwrap_err()
                .starts_with("program artifact:")
        );
    }
    let mut ar = Arena::new();
    let q = quote(&mut ar, 14);
    let compiler = program_record_for_profile(&mut ar, q);
    assert!(
        run(encoded(&ar, compiler), input.clone(), RunLimits::default())
            .unwrap_err()
            .contains("compiler job admission")
    );
    assert!(run(input.clone(), input.clone(), RunLimits::default())
        .unwrap_err()
        .contains("ART1"));
    for tag in [16, 17] {
        let (p, i) = fixture(|ar| {
            let zero = atom(ar, 0);
            (op(ar, tag, zero), zero)
        });
        assert!(run(p, i, RunLimits::default())
            .unwrap_err()
            .contains("UnsupportedService"));
    }
}
fn program_record_for_profile(ar: &mut Arena, formula: Order) -> Order {
    program(ar, formula, 1)
}

#[test]
fn hard_limits_and_expired_deadlines_are_explicit() {
    for limits in [
        RunLimits {
            budget: 0,
            ..RunLimits::default()
        },
        RunLimits {
            budget: u64::MAX,
            ..RunLimits::default()
        },
        RunLimits {
            arena_nodes: 196_609,
            ..RunLimits::default()
        },
        RunLimits {
            frames: 65_537,
            ..RunLimits::default()
        },
        RunLimits {
            artifact_bytes: (16 << 20) + 1,
            ..RunLimits::default()
        },
        RunLimits {
            artifact_nodes: 196_609,
            ..RunLimits::default()
        },
        RunLimits {
            artifact_depth: 4097,
            ..RunLimits::default()
        },
        RunLimits {
            time_ms: 60_001,
            ..RunLimits::default()
        },
    ] {
        assert!(run(Vec::new(), Vec::new(), limits)
            .unwrap_err()
            .starts_with("limit "));
    }
    assert!(deadline(
        Instant::now() - Duration::from_secs(1),
        RunLimits {
            time_ms: 1,
            ..RunLimits::default()
        }
    )
    .unwrap_err()
    .contains("deadline"));
}
