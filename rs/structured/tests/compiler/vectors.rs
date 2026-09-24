use super::*;
use serde_json::{json, Value};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn cli_vectors_bind_actual_guest_results_to_independent_canonical_files() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let good = compiler(&mut ar, generated, 0, None);
    let input = make_job(&mut ar, good, schema::FIXTURE_CAPS);
    let expected = schema::success(&mut ar, input, generated).unwrap();
    let zero = atom(&mut ar, 0);
    let fourteen = atom(&mut ar, 14);
    let mut files = serde_json::Map::new();
    for (name, node) in [
        ("compiler", good),
        ("job", input),
        ("result", expected),
        ("generated", generated),
        ("zero", zero),
        ("fourteen", fourteen),
    ] {
        files.insert(name.into(), Value::String(hex(&encoded(&ar, node))));
    }
    assert_eq!(
        execute_job(&ar, good, input).unwrap().output,
        encoded(&ar, expected)
    );
    let mut cases = vec![
        json!({"name":"success", "program":"compiler", "input":"job", "result":"result", "generated":"generated"}),
    ];
    let d = diagnostic(&mut ar, 6, 0, 0, 0, b"unsupported fixture");
    let diagnostics = schema::seq(&mut ar, &[d, d]).unwrap();
    let error_compiler = compiler(&mut ar, diagnostics, 1, None);
    let error_job = make_job(&mut ar, error_compiler, schema::FIXTURE_CAPS);
    let error_result = schema::failure(
        &mut ar,
        error_job,
        &vec![
            schema::Diagnostic {
                code: 6,
                module: 0,
                start: 0,
                end: 0,
                message: "unsupported fixture".into()
            };
            2
        ],
    )
    .unwrap();
    for (name, node) in [
        ("diagnostic-compiler", error_compiler),
        ("diagnostic-job", error_job),
        ("diagnostic-result", error_result),
    ] {
        files.insert(name.into(), Value::String(hex(&encoded(&ar, node))));
    }
    assert_eq!(
        execute_job(&ar, error_compiler, error_job).unwrap().output,
        encoded(&ar, error_result)
    );
    cases.push(json!({"name":"diagnostics", "program":"diagnostic-compiler", "input":"diagnostic-job", "result":"diagnostic-result"}));
    for kind in 0..6 {
        let (name, compiler, job, message) = match kind {
            0 => {
                let mut limits = schema::FIXTURE_CAPS;
                limits[8] = 1;
                (
                    "budget",
                    good,
                    make_job(&mut ar, good, limits),
                    "budget exhausted",
                )
            }
            1 => {
                let bad = compiler(&mut ar, generated, 0, Some(3));
                (
                    "result-binding",
                    bad,
                    make_job(&mut ar, bad, schema::FIXTURE_CAPS),
                    "job identity mismatch",
                )
            }
            2 => (
                "compiler-binding",
                good,
                error_job,
                "compiler identity mismatch",
            ),
            3 => {
                let formula = op(&mut ar, 16, zero);
                let bad = program(&mut ar, formula, 1);
                (
                    "host-service",
                    bad,
                    make_job(&mut ar, bad, schema::FIXTURE_CAPS),
                    "UnsupportedService",
                )
            }
            4 => {
                let mut limits = schema::FIXTURE_CAPS;
                limits[2] = 1;
                (
                    "diagnostic-cap",
                    error_compiler,
                    make_job(&mut ar, error_compiler, limits),
                    "collection length limit",
                )
            }
            _ => ("schema", good, zero, "compiler job admission"),
        };
        assert!(execute_job(&ar, compiler, job)
            .unwrap_err()
            .contains(message));
        let p = format!("{name}-compiler");
        let i = format!("{name}-job");
        files.insert(p.clone(), Value::String(hex(&encoded(&ar, compiler))));
        files.insert(i.clone(), Value::String(hex(&encoded(&ar, job))));
        cases.push(json!({"name":name,"program":p,"input":i,"error":message}));
    }
    // Explicit opt-in fixture regeneration; expected bytes above come from the
    // independent schema/model, not production result serialization.
    let vectors = json!({"files":files,"cases":cases});
    if let Some(path) = std::env::var_os("JOY_JOB_VECTORS") {
        std::fs::write(path, serde_json::to_vec_pretty(&vectors).unwrap()).unwrap();
    } else {
        let checked: Value =
            serde_json::from_str(include_str!("../../../../cli/tests/compiler_vectors.json"))
                .unwrap();
        assert_eq!(vectors, checked, "canonical CLI fixture drift");
    }
}
