use super::*;

#[path = "../../../../../trident/examples/selfhost_data/model.rs"]
#[allow(dead_code)]
mod data;
#[path = "../../../../../trident/examples/selfhost_jobs/schema.rs"]
#[allow(dead_code)]
mod schema;

fn job(ar: &mut Arena, compiler: u32, source: &[u8]) -> u32 {
    let modules = [schema::Module {
        path: "demo".into(),
        origin: "fixture".into(),
        version: "1".into(),
        source: source.to_vec(),
    }];
    let options = schema::Options {
        input: 0,
        output: 0,
        optimization: 0,
        cfg: vec!["release".into()],
    };
    let mut limits = schema::FIXTURE_CAPS;
    limits[5] = host().artifact_bytes as u64;
    limits[6] = u64::from(host().artifact_nodes);
    limits[8] = host().budget;
    limits[9] = u64::from(host().arena_nodes);
    limits[10] = u64::from(host().frames);
    schema::job(ar, compiler, &modules, "demo", "main", &options, &limits).unwrap()
}

// Only the three rules used by this hand-authored fixture. No nox evaluator or
// Joy producer participates in constructing its authenticated record sequence.
fn records(ar: &mut Arena, object: u32, formula: u32, out: &mut Vec<Record>) -> u32 {
    out.push(enter(object, formula));
    let head = ar.head(formula).unwrap();
    let body = ar.tail(formula).unwrap();
    let result = match ar.atom_value(head).unwrap().as_u64() {
        0 => {
            assert_eq!(ar.atom_value(body).unwrap(), F::ZERO);
            let hash = *ar.digest(object).unwrap();
            ar.hash_data(&hash).unwrap()
        }
        1 => body,
        3 => {
            let left = ar.head(body).unwrap();
            let right = ar.tail(body).unwrap();
            let left = records(ar, object, left, out);
            let right = records(ar, object, right, out);
            ar.pair(left, right).unwrap()
        }
        _ => panic!("fixture rule"),
    };
    out.push(finish(result));
    result
}

fn fixture(attack: u8) -> (Fixture, u32) {
    let mut ar = Arena::try_new_boxed().unwrap();
    let zero = atom(&mut ar, 0);
    let (generated, _) = quote(&mut ar, 14);
    let generated = art(&mut ar, generated, if attack == 3 { 1 } else { 0 });
    let payload = if attack == 2 {
        schema::seq(&mut ar, &[]).unwrap()
    } else {
        generated
    };
    let payload = op(&mut ar, 1, payload);
    let identity = if attack == 1 {
        let hash = ar.hash_data(&[F::ZERO; 4]).unwrap();
        op(&mut ar, 1, hash)
    } else {
        op(&mut ar, 0, zero)
    };
    let (status, _) = quote(
        &mut ar,
        if attack == 2 {
            1
        } else if attack == 4 {
            2
        } else {
            0
        },
    );
    let (mut body, _) = quote(&mut ar, 0);
    for child in [payload, status, identity] {
        body = binary(&mut ar, 3, child, body);
    }
    let (tag, _) = quote(&mut ar, schema::RESULT);
    let formula = binary(&mut ar, 3, tag, body);
    let program = art(&mut ar, formula, 1);
    let object = job(&mut ar, program, b"fn main() {}\n");
    let mut actions = Vec::new();
    let result = records(&mut ar, object, formula, &mut actions);
    let cost = actions.len() as u64 / 2;
    (
        Fixture {
            ar,
            program,
            object,
            formula,
            result,
            cost,
            actions,
            profile: 1,
        },
        generated,
    )
}

#[test]
fn valid_computation_must_also_pass_job_bound_res1_admission() {
    let (f, generated) = fixture(0);
    let done = f.verify(&f.payload(&f.records())).unwrap();
    assert_eq!(done.compiled, Some(encode(&f.ar, generated)));
    assert_eq!(done.report.compiler_job.unwrap().status, "success");
    for (attack, error) in [
        (1, "job identity mismatch"),
        (2, "empty compile-error"),
        (3, "profile mismatch"),
        (4, "result status"),
    ] {
        let (f, _) = fixture(attack);
        let actual = f.verify(&f.payload(&f.records())).unwrap_err();
        assert!(actual.contains(error), "attack {attack}: {actual}");
    }
}

#[test]
fn job_source_identity_is_bound_even_after_valid_transport_rechaining() {
    let (mut f, _) = fixture(0);
    let changed = job(&mut f.ar, f.program, b"different source\n");
    let payload = f.payload(&f.records());
    let program = encode(&f.ar, f.program);
    let input = encode(&f.ar, changed);
    let original = envelope(&payload, f.context());
    assert!(
        pipeline::verify::<4096, _>(&program, &input, Cursor::new(original), host(), caps())
            .unwrap_err()
            .contains("context")
    );
    let mut context = f.context();
    context.object = particle(&f.ar, changed);
    let rechained = envelope(&payload, context);
    assert!(
        pipeline::verify::<4096, _>(&program, &input, Cursor::new(rechained), host(), caps())
            .unwrap_err()
            .contains("Key")
    );
}

#[test]
fn invalid_job_and_wrong_compiler_identity_reject_before_semantic_certificate() {
    let (mut f, _) = fixture(0);
    let raw = atom(&mut f.ar, 0);
    let (other, _) = quote(&mut f.ar, 9);
    let other = art(&mut f.ar, other, 1);
    let wrong = job(&mut f.ar, other, b"fn main() {}\n");
    for input in [raw, wrong] {
        let error = pipeline::verify::<4096, _>(
            &encode(&f.ar, f.program),
            &encode(&f.ar, input),
            Cursor::new(Vec::<u8>::new()),
            host(),
            caps(),
        )
        .unwrap_err();
        assert!(!error.contains("certificate transport"));
    }
}
