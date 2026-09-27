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
