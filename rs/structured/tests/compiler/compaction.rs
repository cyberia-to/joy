use super::*;

#[test]
fn compiler_job_binding_and_extracted_artifact_survive_repeated_collection() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let (loop_body, loop_input) = loop_fixture(&mut ar, 200, false);
    let subject = quoted(&mut ar, loop_input);
    let code = quoted(&mut ar, loop_body);
    let work = binary(&mut ar, 2, subject, code);
    let payload = quoted(&mut ar, generated);
    let payload_code = quoted(&mut ar, payload);
    let payload = binary(&mut ar, 2, work, payload_code);
    let compiler = compiler_expression(&mut ar, payload, 0, None);
    let mut caps = schema::FIXTURE_CAPS;
    caps[8] = 100_000;
    caps[9] = 10_000;
    let job = make_job(&mut ar, compiler, caps);
    let limits = RunLimits {
        compaction: Some(CompactionPolicy {
            resident_nodes: 512,
            collection_work: 100_000_000,
        }),
        ..RunLimits::default()
    };
    let result = run(encoded(&ar, compiler), encoded(&ar, job), limits).unwrap();
    let ordinary = execute_job(&ar, compiler, job).unwrap();
    assert_eq!(result.output, ordinary.output);
    assert_eq!(result.compiled, Some(encoded(&ar, generated)));
    assert_eq!(
        result.report.charged_reductions,
        ordinary.report.charged_reductions
    );
    for time_ms in [3_600_000, 7_200_000] {
        let at_deadline = run(
            encoded(&ar, compiler),
            encoded(&ar, job),
            RunLimits { time_ms, ..limits },
        )
        .unwrap();
        assert_eq!(at_deadline.output, result.output);
        assert_eq!(at_deadline.compiled, result.compiled);
        let mut expected = serde_json::to_value(&result.report).unwrap();
        let mut actual = serde_json::to_value(&at_deadline.report).unwrap();
        expected.as_object_mut().unwrap().remove("elapsed_micros");
        actual.as_object_mut().unwrap().remove("elapsed_micros");
        assert_eq!(actual, expected);
    }
    let stats = result.report.compaction.unwrap();
    assert!(stats.collections > 1);
    assert!(stats.reclaimed_nodes > 512);
    assert_eq!(stats.allocation_limit, 10_000);
    assert_eq!(stats.resident_limit, 512);
    // The job can tighten cumulative work independently of the host ceiling.
    caps[9] = 512;
    let tight = make_job(&mut ar, compiler, caps);
    let error = run(encoded(&ar, compiler), encoded(&ar, tight), limits).unwrap_err();
    assert!(error.contains("TotalAllocations"), "{error}");
}

#[test]
fn larger_host_allowance_still_admits_and_enforces_explicit_job_reductions() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let compiler = compiler(&mut ar, generated, 0, None);
    let limits = RunLimits {
        budget: 20_000_000_000,
        compaction: Some(CompactionPolicy {
            resident_nodes: 512,
            collection_work: 100_000_000,
        }),
        ..RunLimits::default()
    };
    let mut caps = schema::FIXTURE_CAPS;
    let ordinary_job = make_job(&mut ar, compiler, caps);
    let ordinary = execute_job(&ar, compiler, ordinary_job).unwrap();
    for budget in [caps[8], 10_000_000_000, 20_000_000_000] {
        caps[8] = budget;
        let job = make_job(&mut ar, compiler, caps);
        let result = run(encoded(&ar, compiler), encoded(&ar, job), limits).unwrap();
        assert_eq!(result.compiled, ordinary.compiled);
        assert_eq!(
            result.report.charged_reductions,
            ordinary.report.charged_reductions
        );
    }

    // A larger JOB1 cannot enlarge the explicitly selected host allowance.
    let large_job = make_job(&mut ar, compiler, caps);
    let error = run(
        encoded(&ar, compiler),
        encoded(&ar, large_job),
        RunLimits {
            budget: 10_000_000_000,
            ..limits
        },
    )
    .unwrap_err();
    assert!(error.contains("compiler job admission"), "{error}");

    // Conversely, a small JOB1 still tightens the larger host allowance.
    caps[8] = ordinary.report.charged_reductions - 1;
    let tight_job = make_job(&mut ar, compiler, caps);
    let error = run(encoded(&ar, compiler), encoded(&ar, tight_job), limits).unwrap_err();
    assert!(error.contains("budget exhausted"), "{error}");
}
