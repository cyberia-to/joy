use super::*;

#[path = "../../../../trident/examples/selfhost_data/model.rs"]
#[allow(dead_code)]
mod data;
#[path = "../../../../trident/examples/selfhost_jobs/validate.rs"]
#[allow(dead_code)]
mod reference;
#[path = "../../../../trident/examples/selfhost_jobs/schema.rs"]
#[allow(dead_code)]
mod schema;

mod quotas;
mod rejection;
mod vectors;

fn configuration() -> (Vec<schema::Module>, schema::Options) {
    (
        vec![schema::Module {
            path: "demo".into(),
            origin: "fixture".into(),
            version: "1".into(),
            source: b"raw\0\r\n\xff".to_vec(),
        }],
        schema::Options {
            input: 0,
            output: 0,
            optimization: 0,
            cfg: vec!["release".into()],
        },
    )
}

fn quoted(ar: &mut Arena, noun: Order) -> Order {
    op(ar, 1, noun)
}

fn generated_program(ar: &mut Arena) -> Order {
    let q = quote(ar, 14);
    program(ar, q, 0)
}

// This fixture constructs RES1 during real nox execution. It does not compile
// source: its ART1 payload is quoted data. SH2 separately requires a compiler.
fn compiler(ar: &mut Arena, payload: Order, status: u64, corrupt_limb: Option<usize>) -> Order {
    let payload = quoted(ar, payload);
    compiler_expression(ar, payload, status, corrupt_limb)
}

fn compiler_expression(
    ar: &mut Arena,
    payload: Order,
    status: u64,
    corrupt_limb: Option<usize>,
) -> Order {
    let identity = if let Some(index) = corrupt_limb {
        let one = quote(ar, 1);
        let mut limbs = [0; 4];
        for (i, limb) in limbs.iter_mut().enumerate() {
            *limb = axis(ar, 4 + i as u64);
            if i == index {
                *limb = binary(ar, 5, *limb, one);
            }
        }
        let left = binary(ar, 3, limbs[0], limbs[1]);
        let right = binary(ar, 3, limbs[2], limbs[3]);
        let changed = binary(ar, 3, left, right);
        let hash = axis(ar, 0);
        let code = quoted(ar, changed);
        binary(ar, 2, hash, code)
    } else {
        axis(ar, 0)
    };
    let mut body = quote(ar, 0);
    let status = quote(ar, status);
    for field in [payload, status, identity] {
        body = binary(ar, 3, field, body);
    }
    let tag = quote(ar, schema::RESULT);
    let formula = binary(ar, 3, tag, body);
    program(ar, formula, 1)
}

#[test]
fn generated_compiler_profile_is_admitted_only_when_requested_explicitly() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let inner = compiler(&mut ar, generated, 0, None);
    let outer = compiler(&mut ar, inner, 0, None);
    let (modules, mut options) = configuration();
    options.input = 1;
    options.output = 1;
    let job = schema::job(
        &mut ar,
        outer,
        &modules,
        "demo",
        "main",
        &options,
        &schema::FIXTURE_CAPS,
    )
    .unwrap();
    let result = execute_job(&ar, outer, job).unwrap();
    assert_eq!(result.compiled.unwrap(), encoded(&ar, inner));
    let inner_job = make_job(&mut ar, inner, schema::FIXTURE_CAPS);
    assert_eq!(
        execute_job(&ar, inner, inner_job)
            .unwrap()
            .compiled
            .unwrap(),
        encoded(&ar, generated)
    );
}

fn make_job(ar: &mut Arena, compiler: Order, limits: [u64; 11]) -> Order {
    let (modules, options) = configuration();
    schema::job(ar, compiler, &modules, "demo", "main", &options, &limits).unwrap()
}

fn execute_job(ar: &Arena, compiler: Order, job: Order) -> Result<RunResult, String> {
    run(
        encoded(ar, compiler),
        encoded(ar, job),
        RunLimits::default(),
    )
}

fn fields(ar: &Arena, root: Order, count: usize) -> Vec<Order> {
    let mut rest = ar.tail(root).unwrap();
    (0..count)
        .map(|_| {
            let field = ar.head(rest).unwrap();
            rest = ar.tail(rest).unwrap();
            field
        })
        .collect()
}

fn replace(ar: &mut Arena, root: Order, count: usize, index: usize, value: Order) -> Order {
    let tag = ar.atom_value(ar.head(root).unwrap()).unwrap().as_u64();
    let mut fields = fields(ar, root, count);
    fields[index] = value;
    schema::record(ar, tag, &fields).unwrap()
}

fn diagnostic(
    ar: &mut Arena,
    code: u64,
    module: u64,
    start: u64,
    end: u64,
    message: &[u8],
) -> Order {
    let fields = [
        atom(ar, code),
        atom(ar, module),
        atom(ar, start),
        atom(ar, end),
        schema::bytes(ar, message).unwrap(),
    ];
    schema::record(ar, schema::DIAGNOSTIC, &fields).unwrap()
}

#[test]
fn guest_constructed_result_matches_independent_reference_and_program_executes() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let compiler = compiler(&mut ar, generated, 0, None);
    let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
    let result = execute_job(&ar, compiler, job).unwrap();
    let expected = schema::success(&mut ar, job, generated).unwrap();
    assert_eq!(result.output, encoded(&ar, expected));
    assert_eq!(result.compiled.as_ref().unwrap(), &encoded(&ar, generated));
    let report = result.report.compiler_job.as_ref().unwrap();
    assert_eq!(report.status, "success");
    assert_eq!(report.modules[0].source_bytes, 7);
    assert_eq!(report.options.cfg_flags, ["release"]);
    let admitted = reference::job(&mut ar, job, compiler, schema::FIXTURE_CAPS).unwrap();
    assert_eq!(admitted.modules(), configuration().0);
    assert!(
        matches!(reference::result(&mut ar, expected, job, &admitted).unwrap(),
        reference::ResultValue::Success { artifact, .. } if artifact == generated)
    );
    let zero = atom(&mut ar, 0);
    let executed = run(
        result.compiled.unwrap(),
        encoded(&ar, zero),
        RunLimits::default(),
    )
    .unwrap();
    assert_eq!(output_atom(&executed.output), 14);
}

#[test]
fn changed_bindings_and_sufficient_limits_preserve_generated_program_bytes() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let first = compiler(&mut ar, generated, 0, None);
    // A real, distinct compiler formula uses an extra composition.
    let original = fields(&ar, first, 4)[3];
    let input = axis(&mut ar, 1);
    let code = quoted(&mut ar, original);
    let formula = binary(&mut ar, 2, input, code);
    let second = program(&mut ar, formula, 1);
    let mut roots = Vec::new();
    for (compiler, budget) in [(first, 1_000_000), (second, 500_000)] {
        let mut limits = schema::FIXTURE_CAPS;
        limits[8] = budget;
        let job = make_job(&mut ar, compiler, limits);
        let result = execute_job(&ar, compiler, job).unwrap();
        assert_eq!(result.compiled.unwrap(), encoded(&ar, generated));
        roots.push((
            result.report.program_particle,
            result.report.input_particle,
            result.report.output_particle,
        ));
    }
    assert_ne!(roots[0].0, roots[1].0);
    assert_ne!(roots[0].1, roots[1].1);
    assert_ne!(roots[0].2, roots[1].2);
}

#[test]
fn compile_errors_are_valid_results_with_ordered_utf8_diagnostics_and_no_program() {
    let mut ar = Arena::new();
    let d = diagnostic(&mut ar, 1, 0, 7, 7, "ошибка".as_bytes());
    let payload = schema::seq(&mut ar, &[d, d]).unwrap();
    let compiler = compiler(&mut ar, payload, 1, None);
    let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
    let result = execute_job(&ar, compiler, job).unwrap();
    assert!(result.compiled.is_none());
    let report = result.report.compiler_job.unwrap();
    assert_eq!(report.status, "compile_error");
    assert_eq!(report.diagnostics.len(), 2);
    assert_eq!(report.diagnostics[0], report.diagnostics[1]);
    let expected = schema::failure(
        &mut ar,
        job,
        &vec![
            schema::Diagnostic {
                code: 1,
                module: 0,
                start: 7,
                end: 7,
                message: "ошибка".into()
            };
            2
        ],
    )
    .unwrap();
    assert_eq!(result.output, encoded(&ar, expected));
}

#[test]
fn trident_source_seed_exports_compiler_profile_and_executes_bound_job_on_nox() {
    let source = include_str!("../../tests/fixtures/compiler_transport.tri");
    let seed = trident::compile_native_artifact(
        source,
        "compiler_transport.tri",
        &trident::CompileOptions::default(),
        trident::NativeArtifactProfile::CompilerJob,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .unwrap();
    let mut ar = Arena::new();
    let compiler =
        artifact::decode(&mut ar, &seed.bytes, RunLimits::default().transport()).unwrap();
    let generated = generated_program(&mut ar);
    let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
    let result = run(seed.bytes.clone(), encoded(&ar, job), RunLimits::default()).unwrap();
    assert_eq!(result.compiled.as_ref().unwrap(), &encoded(&ar, generated));
    let expected = schema::success(&mut ar, job, generated).unwrap();
    assert_eq!(result.output, encoded(&ar, expected));
    let zero = atom(&mut ar, 0);
    if let Some(path) = std::env::var_os("JOY_SEED_FIXTURES") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("compiler.tri"), source).unwrap();
        std::fs::write(path.join("compiler.dag"), &seed.bytes).unwrap();
        for (name, root) in [
            ("job", job),
            ("expected-result", expected),
            ("expected-program", generated),
            ("zero", zero),
        ] {
            std::fs::write(path.join(format!("{name}.dag")), encoded(&ar, root)).unwrap();
        }
        std::fs::write(
            path.join("report.json"),
            serde_json::to_vec_pretty(&result.report).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        output_atom(
            &run(
                result.compiled.unwrap(),
                encoded(&ar, zero),
                RunLimits::default()
            )
            .unwrap()
            .output
        ),
        14
    );
}
