use super::*;

fn attempt(limits: [u64; 11]) -> Result<RunResult, String> {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let compiler = compiler(&mut ar, generated, 0, None);
    let job = make_job(&mut ar, compiler, limits);
    execute_job(&ar, compiler, job)
}

#[test]
fn every_job_limit_is_positive_and_cannot_raise_independent_host_caps() {
    let host = RunLimits::default();
    let c = host.compiler;
    let caps = [
        c.source_bytes as u64,
        c.modules as u64,
        c.diagnostics as u64,
        c.sequence_length as u64,
        c.validation_visits as u64,
        host.artifact_bytes as u64,
        host.artifact_nodes as u64,
        host.artifact_depth as u64,
        host.budget,
        host.arena_nodes as u64,
        host.frames as u64,
    ];
    for i in 0..11 {
        for value in [0, caps[i] + 1] {
            let mut limits = schema::FIXTURE_CAPS;
            limits[i] = value;
            assert!(
                attempt(limits)
                    .unwrap_err()
                    .contains("unsupported job limit"),
                "field {i}"
            );
        }
    }
    for (i, value) in [(0, 6), (4, 1), (5, 100), (6, 1), (7, 1), (9, 1)] {
        let mut limits = schema::FIXTURE_CAPS;
        limits[i] = value;
        assert!(attempt(limits).is_err(), "field {i}");
    }
}

#[test]
fn requested_execution_and_shared_visit_limits_are_used_exactly() {
    let baseline = attempt(schema::FIXTURE_CAPS).unwrap();
    let visits = baseline
        .report
        .compiler_job
        .as_ref()
        .unwrap()
        .input_validation_visits;
    for (field, exact, error) in [
        (0, 7, "collection length"),
        (4, visits as u64, "visits"),
        (8, baseline.report.charged_reductions, "budget exhausted"),
        (10, baseline.report.peak_frames as u64, "Frames"),
    ] {
        let mut limits = schema::FIXTURE_CAPS;
        limits[field] = exact;
        let result = attempt(limits).unwrap();
        assert_eq!(result.compiled, baseline.compiled);
        limits[field] -= 1;
        assert!(
            attempt(limits).unwrap_err().contains(error),
            "field {field}"
        );
    }
    // The JOB1 identity changes with the requested cap. Stabilize its actual
    // allocation count before testing this cap's inclusive boundary.
    let mut limits = schema::FIXTURE_CAPS;
    let mut exact = baseline.report.allocated_nodes;
    for _ in 0..8 {
        limits[9] = exact as u64;
        match attempt(limits) {
            Ok(result) if result.report.allocated_nodes == exact => break,
            Ok(result) => exact = result.report.allocated_nodes,
            Err(_) => exact += 1,
        }
    }
    limits[9] = exact as u64;
    let result = attempt(limits).unwrap();
    assert_eq!(result.report.allocated_nodes, exact);
    limits[9] -= 1;
    assert!(attempt(limits).is_err());
}

#[test]
fn aggregate_source_and_repeated_records_share_one_admission_allowance() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let compiler = compiler(&mut ar, generated, 0, None);
    let (mut modules, options) = configuration();
    let mut m = modules[0].clone();
    m.path = "extra".into();
    modules.push(m);
    let mut limits = schema::FIXTURE_CAPS;
    limits[0] = 14;
    let job = schema::job(
        &mut ar, compiler, &modules, "demo", "main", &options, &limits,
    )
    .unwrap();
    let baseline = execute_job(&ar, compiler, job).unwrap();
    let visits = baseline
        .report
        .compiler_job
        .unwrap()
        .input_validation_visits;
    for (field, value) in [(0, 13), (1, 1), (3, 1), (4, (visits - 1) as u64)] {
        let mut bad = limits;
        bad[field] = value;
        let job = schema::job(&mut ar, compiler, &modules, "demo", "main", &options, &bad).unwrap();
        assert!(execute_job(&ar, compiler, job).is_err(), "field {field}");
    }
}

#[test]
fn output_diagnostics_have_a_separate_shared_allowance_and_count_limit() {
    let mut ar = Arena::new();
    let d = diagnostic(&mut ar, 6, 0, 0, 0, &vec![b'x'; 2048]);
    let payload = schema::seq(&mut ar, &[d, d]).unwrap();
    let compiler = compiler(&mut ar, payload, 1, None);
    let mut limits = schema::FIXTURE_CAPS;
    let job = make_job(&mut ar, compiler, limits);
    let report = execute_job(&ar, compiler, job)
        .unwrap()
        .report
        .compiler_job
        .unwrap();
    assert!(report.output_validation_visits > report.input_validation_visits);
    limits[4] = report.output_validation_visits as u64;
    limits[2] = 2;
    let job = make_job(&mut ar, compiler, limits);
    execute_job(&ar, compiler, job).unwrap();
    limits[4] -= 1;
    let job = make_job(&mut ar, compiler, limits);
    assert!(execute_job(&ar, compiler, job)
        .unwrap_err()
        .contains("compiler result"));
    limits[4] += 1;
    limits[2] = 1;
    let job = make_job(&mut ar, compiler, limits);
    assert!(execute_job(&ar, compiler, job)
        .unwrap_err()
        .contains("compiler result"));
}

#[test]
fn shared_logical_trees_and_noncanonical_empty_padding_cannot_bypass_admission() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let compiler = compiler(&mut ar, generated, 0, None);
    let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
    let job_fields = fields(&ar, job, 6);
    let options = job_fields[4];
    let leaf = schema::bytes(&mut ar, b"a").unwrap();
    let mut tree = leaf;
    for _ in 0..12 {
        tree = ar.pair(tree, tree).unwrap();
    }
    let length = atom(&mut ar, 4096);
    let body = ar.pair(length, tree).unwrap();
    let flags = op(&mut ar, data::SEQ, body);
    let options = replace(&mut ar, options, 5, 4, flags);
    let job = replace(&mut ar, job, 6, 4, options);
    assert!(execute_job(&ar, compiler, job).is_err());
    let zero = atom(&mut ar, 0);
    let bad = ar.pair(zero, leaf).unwrap();
    let flags = op(&mut ar, data::SEQ, bad);
    let options = replace(&mut ar, options, 5, 4, flags);
    let job = replace(&mut ar, job, 6, 4, options);
    assert!(execute_job(&ar, compiler, job)
        .unwrap_err()
        .contains("padding"));
}

#[test]
fn requested_transport_caps_cover_both_compiler_and_job_at_inclusive_boundaries() {
    // Every attempt has the same logical input and generated program. The
    // changed LIM1 is identity-bound; find and check each inclusive cutoff.
    for field in [5, 6, 7] {
        let mut low = 1;
        let mut high = schema::FIXTURE_CAPS[field];
        while low < high {
            let mid = low + (high - low) / 2;
            let mut limits = schema::FIXTURE_CAPS;
            limits[field] = mid;
            if attempt(limits).is_ok() {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let mut limits = schema::FIXTURE_CAPS;
        limits[field] = high;
        attempt(limits).unwrap();
        limits[field] -= 1;
        assert!(
            attempt(limits)
                .unwrap_err()
                .contains("requested artifact limits"),
            "field {field}"
        );
    }
}

#[test]
fn runtime_constructed_res1_obeys_requested_transport_caps_before_extraction() {
    let mut ar = Arena::new();
    let (loop_code, loop_input) = loop_fixture(&mut ar, 256, true);
    let subject = quoted(&mut ar, loop_input);
    let code = quoted(&mut ar, loop_code);
    let chain = binary(&mut ar, 2, subject, code);
    // Guest constructs ART1 whose formula quotes the newly built chain.
    let q1 = quote(&mut ar, 1);
    let formula = binary(&mut ar, 3, q1, chain);
    let zero = quote(&mut ar, 0);
    let mut body = zero;
    for field in [formula, zero, zero, zero] {
        body = binary(&mut ar, 3, field, body);
    }
    let tag = quote(&mut ar, schema::ARTIFACT);
    let payload = binary(&mut ar, 3, tag, body);
    let compiler = compiler_expression(&mut ar, payload, 0, None);
    let mut limits = schema::FIXTURE_CAPS;
    limits[7] = 512;
    let job = make_job(&mut ar, compiler, limits);
    let result = execute_job(&ar, compiler, job).unwrap();
    let inputs = [encoded(&ar, compiler), encoded(&ar, job)];
    let bytes = inputs.iter().map(Vec::len).max().unwrap() as u64;
    let nodes = inputs
        .iter()
        .map(|b| u32::from_le_bytes(b[40..44].try_into().unwrap()))
        .max()
        .unwrap() as u64;
    assert!(result.output.len() as u64 > bytes);
    for (field, cap) in [(5, bytes + 100), (6, nodes + 10), (7, 64)] {
        let mut bad = limits;
        bad[field] = cap;
        let job = make_job(&mut ar, compiler, bad);
        let error = execute_job(&ar, compiler, job).unwrap_err();
        assert!(
            error.starts_with("compiler result: requested artifact limits"),
            "{field}: {error}"
        );
    }
}

#[test]
fn compiler_container_must_fit_the_job_request_even_when_input_is_smaller() {
    let mut ar = Arena::new();
    let mut chain = atom(&mut ar, 0);
    for i in 0..512 {
        let value = atom(&mut ar, i);
        chain = ar.pair(value, chain).unwrap();
    }
    let formula = quoted(&mut ar, chain);
    let generated = program(&mut ar, formula, 0);
    let compiler = compiler(&mut ar, generated, 0, None);
    let mut limits = schema::FIXTURE_CAPS;
    limits[7] = 1024;
    let job = make_job(&mut ar, compiler, limits);
    execute_job(&ar, compiler, job).unwrap();
    let input = encoded(&ar, job);
    assert!(encoded(&ar, compiler).len() > input.len() + 100);
    let nodes = u32::from_le_bytes(input[40..44].try_into().unwrap()) as u64;
    for (field, cap) in [(5, input.len() as u64 + 100), (6, nodes + 10), (7, 64)] {
        let mut bad = limits;
        bad[field] = cap;
        let job = make_job(&mut ar, compiler, bad);
        let error = execute_job(&ar, compiler, job).unwrap_err();
        assert!(
            error.starts_with(
                "compiler job admission: compiler container: requested artifact limits"
            ),
            "{field}: {error}"
        );
    }
}
