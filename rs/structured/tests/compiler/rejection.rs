use super::*;

#[test]
fn all_four_compiler_and_result_identity_limbs_are_bound() {
    for limb in 0..4 {
        let mut ar = Arena::new();
        let generated = generated_program(&mut ar);
        let good = compiler(&mut ar, generated, 0, None);
        let job = make_job(&mut ar, good, schema::FIXTURE_CAPS);
        let identity = fields(&ar, job, 6)[0];
        let mut digest = ar.read_hash_data(identity).unwrap();
        digest[limb] += Goldilocks::new(1);
        let bad = ar.hash_data(&digest).unwrap();
        let job = replace(&mut ar, job, 6, 0, bad);
        assert!(execute_job(&ar, good, job)
            .unwrap_err()
            .contains("compiler identity mismatch"));
        let bad = compiler(&mut ar, generated, 0, Some(limb));
        let job = make_job(&mut ar, bad, schema::FIXTURE_CAPS);
        assert!(execute_job(&ar, bad, job)
            .unwrap_err()
            .contains("job identity mismatch"));
    }
}

#[test]
fn packages_options_and_entry_are_structurally_validated_without_source_parsing() {
    let (modules, options) = configuration();
    let mut cases = Vec::new();
    for path in ["../demo", "a..b", "1demo", "absent"] {
        let mut m = modules.clone();
        m[0].path = path.into();
        cases.push((m, options.clone(), "demo", "main"));
    }
    for origin in ["", "bad\n", "é"] {
        let mut m = modules.clone();
        m[0].origin = origin.into();
        cases.push((m, options.clone(), "demo", "main"));
    }
    let mut duplicate = modules.clone();
    duplicate.push(modules[0].clone());
    cases.push((duplicate, options.clone(), "demo", "main"));
    let mut unsorted = modules.clone();
    let mut m = modules[0].clone();
    m.path = "aaa".into();
    unsorted.push(m);
    cases.push((unsorted, options.clone(), "demo", "main"));
    for cfg in [vec!["z", "a"], vec!["a", "a"], vec!["a.b"]] {
        let mut o = options.clone();
        o.cfg = cfg.into_iter().map(String::from).collect();
        cases.push((modules.clone(), o, "demo", "main"));
    }
    for (input, output, opt) in [(1, 0, 0), (0, 1, 0), (2, 2, 0), (0, 0, 1)] {
        let mut o = options.clone();
        o.input = input;
        o.output = output;
        o.optimization = opt;
        cases.push((modules.clone(), o, "demo", "main"));
    }
    cases.push((modules.clone(), options.clone(), "demo", "a.b"));
    cases.push((modules, options, "a.demo", "main"));
    for (modules, options, entry, function) in cases {
        let mut ar = Arena::new();
        let generated = generated_program(&mut ar);
        let compiler = compiler(&mut ar, generated, 0, None);
        let job = schema::job(
            &mut ar,
            compiler,
            &modules,
            entry,
            function,
            &options,
            &schema::FIXTURE_CAPS,
        )
        .unwrap();
        assert!(execute_job(&ar, compiler, job)
            .unwrap_err()
            .contains("compiler job admission"));
    }
}

#[test]
fn malformed_records_collections_and_byte_padding_are_rejected() {
    let mut ar = Arena::new();
    let generated = generated_program(&mut ar);
    let compiler = compiler(&mut ar, generated, 0, None);
    let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
    let values = fields(&ar, job, 6);
    for tag in [schema::JOB + 1, schema::JOB] {
        for count in [5, 7] {
            let mut v = values.clone();
            v.push(values[0]);
            let bad = schema::record(&mut ar, tag, &v[..count]).unwrap();
            assert!(execute_job(&ar, compiler, bad).is_err());
        }
    }
    for name in [b"\xff".as_slice(), b"demo\0", b""] {
        let bad = schema::bytes(&mut ar, name).unwrap();
        let bad = replace(&mut ar, job, 6, 2, bad);
        assert!(execute_job(&ar, compiler, bad).is_err());
    }
    let zero = atom(&mut ar, 0);
    let one = atom(&mut ar, 1);
    let tag = atom(&mut ar, data::BYTES);
    for (length, tree) in [(0, one), (1, atom(&mut ar, 256)), (5, one)] {
        let len = atom(&mut ar, length);
        let body = ar.pair(len, tree).unwrap();
        let bad = ar.pair(tag, body).unwrap();
        let bad = replace(&mut ar, job, 6, 2, bad);
        assert!(execute_job(&ar, compiler, bad).is_err());
    }
    let options = fields(&ar, values[4], 5);
    let pair_len = ar.pair(zero, zero).unwrap();
    let body = ar.pair(pair_len, zero).unwrap();
    let bad = op(&mut ar, data::SEQ, body);
    let options = schema::record(
        &mut ar,
        schema::OPTIONS,
        &[options[0], options[1], options[2], options[3], bad],
    )
    .unwrap();
    let bad = replace(&mut ar, job, 6, 4, options);
    assert!(execute_job(&ar, compiler, bad).is_err());
}

#[test]
fn diagnostic_codes_spans_order_utf8_and_overflow_marker_are_enforced() {
    for case in 0..12 {
        let mut ar = Arena::new();
        let a = diagnostic(&mut ar, 1, 0, 0, 0, b"a");
        let b = diagnostic(&mut ar, 1, 0, 0, 0, b"b");
        let overflow = diagnostic(&mut ar, 8, 0, 0, 0, b"limit");
        let nodes = match case {
            0 => vec![],
            1 => vec![b, a],
            2 => vec![diagnostic(&mut ar, 0, 0, 0, 0, b"bad")],
            3 => vec![diagnostic(&mut ar, 9, 0, 0, 0, b"bad")],
            4 => vec![diagnostic(&mut ar, 1, 1, 0, 0, b"bad")],
            5 => vec![diagnostic(&mut ar, 1, 0, 3, 2, b"bad")],
            6 => vec![diagnostic(&mut ar, 1, 0, 0, 8, b"bad")],
            7 => vec![diagnostic(&mut ar, 1, 0, 0, 0, b"\xff")],
            8 => vec![diagnostic(&mut ar, 8, 0, 1, 1, b"bad")],
            9 => vec![overflow, overflow],
            10 => vec![diagnostic(&mut ar, 1u64 << 32, 0, 0, 0, b"bad")],
            _ => vec![a, overflow],
        };
        let payload = schema::seq(&mut ar, &nodes).unwrap();
        let compiler = compiler(&mut ar, payload, 1, None);
        let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
        let result = execute_job(&ar, compiler, job);
        if case == 11 {
            assert!(result.is_ok());
        } else {
            assert!(
                result.unwrap_err().contains("compiler result"),
                "case {case}"
            );
        }
    }
}

#[test]
fn generated_profile_status_and_runtime_faults_cannot_become_success() {
    for kind in 0..5 {
        let mut ar = Arena::new();
        let payload = if kind == 0 {
            atom(&mut ar, 14)
        } else {
            let q = quote(&mut ar, 14);
            program(&mut ar, q, 1)
        };
        let compiler = if kind < 3 {
            compiler(&mut ar, payload, if kind == 2 { 2 } else { 0 }, None)
        } else {
            let zero = atom(&mut ar, 0);
            let formula = op(&mut ar, if kind == 3 { 16 } else { 17 }, zero);
            program(&mut ar, formula, 1)
        };
        let job = make_job(&mut ar, compiler, schema::FIXTURE_CAPS);
        let error = execute_job(&ar, compiler, job).unwrap_err();
        assert!(
            if kind < 3 {
                error.contains("compiler result")
            } else {
                error.contains("UnsupportedService")
            },
            "{error}"
        );
    }
}
