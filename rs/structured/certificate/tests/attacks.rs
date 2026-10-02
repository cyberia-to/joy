use super::*;

#[test]
fn authenticated_frames_cannot_substitute_child_keys_results_or_order() {
    for tag in [1, 5, 2, 4, 40] {
        let f = fixture(tag);
        let prefix = 1 + f.ar.count() as usize;
        for index in 0..f.actions.len() {
            let attacks = match f.actions[index] {
                Record::Enter { object, formula } => {
                    vec![enter(f.result, formula), enter(object, f.object)]
                }
                Record::Finish { .. } => vec![finish(f.object)],
                _ => unreachable!(),
            };
            for attack in attacks {
                // Compose's continuation already has result as its object.
                if matches!((f.actions[index],attack),(Record::Enter {object:a,formula:b},Record::Enter {object:c,formula:d}) if a==c && b==d)
                {
                    continue;
                }
                if matches!((f.actions[index],attack),(Record::Finish {result:a,..},Record::Finish {result:b,..}) if a==b)
                {
                    continue;
                }
                let mut records = f.records();
                records[prefix + index] = attack;
                let error = f.verify(&f.payload(&records)).unwrap_err();
                assert!(error.contains("semantic"), "tag{tag} index{index}: {error}");
            }
            let mut records = f.records();
            records.remove(prefix + index);
            assert!(
                f.verify(&f.payload(&records)).is_err(),
                "missing action {tag}/{index}"
            );
        }
        let mut records = f.records();
        records.extend_from_slice(&f.actions);
        assert!(f
            .verify(&f.payload(&records))
            .unwrap_err()
            .contains("semantic"));
        if f.actions.len() > 2 {
            let mut records = f.records();
            records.swap(prefix + 1, prefix + 3);
            assert!(f
                .verify(&f.payload(&records))
                .unwrap_err()
                .contains("semantic"));
        }
    }
}

#[test]
fn selected_arms_and_both_computed_continuation_coordinates_are_bound() {
    for tag in [4, 40] {
        let mut f = fixture(tag);
        let (unchosen, _) = quote(&mut f.ar, if tag == 4 { 9 } else { 7 });
        f.actions[3] = enter(f.object, unchosen);
        assert!(f
            .verify(&f.payload(&f.records()))
            .unwrap_err()
            .contains("Key"));
    }
    for coordinate in 0..2 {
        let mut f = fixture(2);
        let Record::Enter { object, formula } = f.actions[5] else {
            unreachable!()
        };
        f.actions[5] = if coordinate == 0 {
            enter(f.object, formula)
        } else {
            enter(object, f.formula)
        };
        assert!(f
            .verify(&f.payload(&f.records()))
            .unwrap_err()
            .contains("Key"));
    }
}

#[test]
fn terminal_particle_cost_complete_output_and_transport_end_are_all_required() {
    let f = fixture(5);
    let records = f.records();
    let mut prefix = Vec::new();
    for r in &records {
        r.write(&mut prefix).unwrap();
    }
    let output = encode(&f.ar, f.result);
    let actual = particle(&f.ar, f.result);
    for limb in 0..4 {
        let mut changed = actual;
        changed[limb] ^= 1;
        let mut payload = prefix.clone();
        terminal(&mut payload, changed, f.cost, &output);
        assert!(f.verify(&payload).unwrap_err().contains("terminal"));
    }
    for cost in [f.cost - 1, f.cost + 1, f.cost ^ (1 << 32), u64::MAX] {
        let mut payload = prefix.clone();
        terminal(&mut payload, actual, cost, &output);
        assert!(f.verify(&payload).unwrap_err().contains("terminal"));
    }
    let mut wrong = prefix.clone();
    terminal(&mut wrong, actual, f.cost, &encode(&f.ar, f.object));
    assert!(f.verify(&wrong).unwrap_err().contains("identity mismatch"));
    let full = f.payload(&records);
    for end in [
        0,
        1,
        prefix.len(),
        prefix.len() + 1,
        prefix.len() + 32,
        prefix.len() + 40,
        prefix.len() + 44,
        full.len() - 1,
    ] {
        assert!(f.verify(&full[..end]).is_err(), "payload prefix {end}");
    }
    for extra in [vec![0], vec![6], vec![255], full.clone()] {
        let mut trailing = full.clone();
        trailing.extend(extra);
        assert!(f.verify(&trailing).unwrap_err().contains("unconsumed"));
    }
    let wire = envelope(&full, f.context());
    for end in [0, 8, 39, wire.len() - 1] {
        assert!(pipeline::verify::<4096, _>(
            &encode(&f.ar, f.program),
            &encode(&f.ar, f.object),
            Cursor::new(wire[..end].to_vec()),
            host(),
            caps()
        )
        .is_err());
    }
    let mut wire = wire;
    wire.push(0);
    assert!(pipeline::verify::<4096, _>(
        &encode(&f.ar, f.program),
        &encode(&f.ar, f.object),
        Cursor::new(wire),
        host(),
        caps()
    )
    .unwrap_err()
    .contains("trailing"));
}

#[test]
fn every_public_context_coordinate_and_expected_artifact_is_bound() {
    let f = fixture(5);
    let payload = f.payload(&f.records());
    for field in 0..3 {
        for limb in 0..4 {
            let mut changed = f.context();
            match field {
                0 => changed.program[limb] ^= 1,
                1 => changed.formula[limb] ^= 1,
                _ => changed.object[limb] ^= 1,
            };
            assert!(f
                .verify_with(&payload, changed, host(), caps())
                .unwrap_err()
                .contains("context"));
        }
    }
    for field in 0..3 {
        let mut changed = f.context();
        match field {
            0 => changed.profile = 1,
            1 => changed.budget += 1,
            _ => changed.frames += 1,
        };
        assert!(f
            .verify_with(&payload, changed, host(), caps())
            .unwrap_err()
            .contains("context"));
    }
    let other = fixture(1);
    let proof = envelope(&payload, f.context());
    for (program, input) in [
        (encode(&other.ar, other.program), encode(&f.ar, f.object)),
        (encode(&f.ar, f.program), encode(&f.ar, f.result)),
    ] {
        assert!(pipeline::verify::<4096, _>(
            &program,
            &input,
            Cursor::new(proof.clone()),
            host(),
            caps()
        )
        .unwrap_err()
        .contains("context"));
    }
    // Rebuild the authenticated context for a changed expected input: the old
    // derivation itself must still be rejected, even for an input-ignoring quote.
    let f = fixture(1);
    let mut changed = f.context();
    changed.object = particle(&f.ar, f.result);
    let wire = envelope(&f.payload(&f.records()), changed);
    assert!(pipeline::verify::<4096, _>(
        &encode(&f.ar, f.program),
        &encode(&f.ar, f.result),
        Cursor::new(wire),
        host(),
        caps()
    )
    .unwrap_err()
    .contains("Key"));
}

#[test]
fn exact_limits_pass_and_one_below_rejects_after_fresh_context_chaining() {
    let f = fixture(5);
    let payload = f.payload(&f.records());
    let host = RunLimits {
        budget: f.cost,
        frames: 2,
        ..host()
    };
    let exact = CertificateLimits {
        nouns: f.ar.count(),
        records: f.records().len() as u64 + 1,
        steps: f.actions.len() as u64,
        ..caps()
    };
    let mut context = f.context();
    context.budget = host.budget;
    context.frames = host.frames;
    f.verify_with(&payload, context, host, exact).unwrap();
    for changed in [
        CertificateLimits {
            nouns: exact.nouns - 1,
            ..exact
        },
        CertificateLimits {
            records: exact.records - 1,
            ..exact
        },
        CertificateLimits {
            steps: exact.steps - 1,
            ..exact
        },
    ] {
        assert!(f.verify_with(&payload, context, host, changed).is_err());
    }
    for changed in [
        RunLimits {
            budget: host.budget - 1,
            ..host
        },
        RunLimits {
            frames: host.frames - 1,
            ..host
        },
    ] {
        let mut context = context;
        context.budget = changed.budget;
        context.frames = changed.frames;
        assert!(f
            .verify_with(&payload, context, changed, exact)
            .unwrap_err()
            .contains("Limit"));
    }
}
