use super::*;
use nox::sequential::observe::CaptureLimits;

fn measured(program: &[u8], input: &[u8], transitions: u64, capacity: usize) -> serde_json::Value {
    driver::execute::<4096>(
        program,
        input,
        RunLimits {
            frames: 4096,
            compaction: Some(CompactionPolicy {
                resident_nodes: 3072,
                collection_work: 1_000_000,
            }),
            ..RunLimits::default()
        },
        Caps {
            transitions,
            nouns: capacity,
            evaluations: capacity,
            frames: 4096,
        },
        CaptureLimits {
            max_events: 100_000,
            max_bytes: 64 << 20,
            max_work: 1_000_000,
        },
    )
    .unwrap()
}

#[test]
fn repeated_evaluations_count_occurrences_and_distinct_keys_separately() {
    let (program, input) = fixture(|ar| {
        let q = quote(ar, 7);
        let z = atom(ar, 0);
        (binary(ar, 5, q, q), z)
    });
    let report = measured(&program, &input, 1000, 1000);
    assert_eq!(report["status"], "completed");
    assert_eq!(report["success"]["charged_reductions"], 3);
    assert_eq!(report["census"]["counts"]["transitions"], 6);
    assert_eq!(report["census"]["counts"]["successful_evaluations"], 3);
    for kind in ["entered_evaluation_keys", "completed_evaluation_keys"] {
        assert_eq!(report["census"][kind]["retained"], 2);
        assert_eq!(report["census"][kind]["hits"], 1);
        assert_eq!(report["census"][kind]["distinct_is_lower_bound"], false);
    }
    assert_eq!(report["census"]["open_invocations"], 0);
    assert_eq!(report["census"]["counts"]["peak_active_invocations"], 2);
}

#[test]
fn saturated_dictionaries_report_lower_bounds_and_preserve_exact_totals() {
    let (program, input) = fixture(|ar| {
        let a = quote(ar, 5);
        let b = quote(ar, 7);
        let z = atom(ar, 0);
        (binary(ar, 5, a, b), z)
    });
    let report = measured(&program, &input, 1000, 1);
    assert_eq!(report["status"], "completed");
    assert_eq!(report["census"]["counts"]["transitions"], 6);
    for kind in [
        "noun_particles",
        "entered_evaluation_keys",
        "completed_evaluation_keys",
    ] {
        assert_eq!(report["census"][kind]["retained"], 1);
        assert_eq!(report["census"][kind]["distinct_is_lower_bound"], true);
    }
    assert_eq!(report["census"]["counts"]["unclassified_enters"], 3);
}

#[test]
fn prefix_ceiling_stops_without_reporting_successful_cost() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 10, false));
    let report = measured(&program, &input, 20, 1000);
    assert_eq!(report["status"], "prefix-ceiling");
    assert_eq!(report["census"]["counts"]["transitions"], 20);
    assert_eq!(report["failure"], "Execution(Execution(Cancelled))");
    assert!(report["success"].is_null());
    assert_eq!(report["census"]["counts"]["completed_events"], 0);
    assert!(report["census"]["open_invocations"].as_u64().unwrap() > 0);
}

#[test]
fn terminal_transition_at_prefix_cap_requires_completed_delivery() {
    let (program, input) = fixture(|ar| {
        let z = atom(ar, 0);
        (quote(ar, 7), z)
    });
    let report = measured(&program, &input, 2, 1000);
    assert_eq!(report["status"], "prefix-ceiling");
    assert_eq!(report["failure"], "Capture(Cancelled)");
    assert_eq!(report["census"]["counts"]["transitions"], 2);
    assert_eq!(report["census"]["open_invocations"], 0);
    assert_eq!(report["census"]["counts"]["completed_events"], 0);
    assert!(report["success"].is_null());
}

#[test]
fn observed_loop_includes_compose_and_branch_continuations() {
    let (program, input) = fixture(|ar| loop_fixture(ar, 10, false));
    let baseline = run(program.clone(), input.clone(), RunLimits::default()).unwrap();
    let report = measured(&program, &input, 1000, 1000);
    assert_eq!(report["status"], "completed");
    assert_eq!(
        report["success"]["charged_reductions"],
        baseline.report.charged_reductions
    );
    assert_eq!(
        report["success"]["result_particle"],
        baseline.report.output_particle
    );
    assert_eq!(report["peak_frames"], baseline.report.peak_frames);
    assert_eq!(
        report["census"]["counts"]["peak_active_invocations"],
        baseline.report.peak_frames
    );
    for index in [3, 4, 5] {
        assert!(
            report["census"]["counts"]["popped_phases"][index]
                .as_u64()
                .unwrap()
                > 0
        );
    }
}

#[test]
fn sink_rejects_conflicting_retained_header_without_publishing_event() {
    let stop = Cell::new(false);
    let mut sink = Census::new(
        Caps {
            transitions: 5,
            nouns: 3,
            evaluations: 3,
            frames: 3,
        },
        &stop,
    )
    .unwrap();
    let begin = Event::Begin {
        version: 1,
        initial: LogicalAction::Enter {
            object: [0; 4],
            formula: [0; 4],
            budget: 3,
        },
        initial_nodes: 1,
        max_frames: 3,
        max_total_allocations: 3,
        max_collection_work: 3,
        resident_limit: 3,
    };
    sink.record(begin).unwrap();
    let node = Node {
        particle: [0; 4],
        value: NodeValue::Atom(1),
        bound: nox::data::Cost::Exact(0),
    };
    sink.record(Event::Node(node)).unwrap();
    let before = sink.summary();
    let bad = Node {
        value: NodeValue::Atom(2),
        ..node
    };
    assert_eq!(
        sink.record(Event::Node(bad)),
        Err("inconsistent retained observation")
    );
    assert_eq!(sink.summary(), before);
}

#[test]
fn capture_cap_failure_preserves_prefix_and_reports_no_success() {
    let (program, input) = fixture(|ar| {
        let z = atom(ar, 0);
        (quote(ar, 7), z)
    });
    let report = driver::execute::<4096>(
        &program,
        &input,
        RunLimits {
            frames: 4096,
            compaction: Some(CompactionPolicy {
                resident_nodes: 3072,
                collection_work: 1000,
            }),
            ..RunLimits::default()
        },
        Caps {
            transitions: 100,
            nouns: 100,
            evaluations: 100,
            frames: 4096,
        },
        CaptureLimits {
            max_events: 1,
            max_bytes: 1000,
            max_work: 1000,
        },
    )
    .unwrap();
    assert_eq!(report["status"], "failed");
    assert_eq!(report["failure"], "Capture(Events)");
    assert_eq!(report["census"]["counts"]["events"], 1);
    assert!(report["success"].is_null());
}
