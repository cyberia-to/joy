use joy_rs::Warrior;
use trident::runtime::{artifact::BundleCost, ProgramBundle, ProgramInput, Runner};

fn bundle(assembly: &str) -> ProgramBundle {
    ProgramBundle {
        name: "secret_stream".into(),
        version: "1".into(),
        target_vm: "nox".into(),
        target_os: None,
        source_hash: "test".into(),
        assembly: assembly.into(),
        entry_point: "main".into(),
        functions: vec![],
        reads_state: false,
        cost: BundleCost {
            table_values: vec![],
            table_names: vec![],
            padded_height: 0,
            estimated_proving_ns: 0,
        },
    }
}

fn input(public: &[u64], secret: &[u64]) -> ProgramInput {
    ProgramInput {
        public: public.to_vec(),
        secret: secret.to_vec(),
        digests: vec![],
    }
}

fn state() -> bbg::certificate::StateCertificate {
    let mut state = bbg::BbgState::new();
    let mut record = bbg::types::ParticleRecord::zero();
    record.energy = 77;
    state.particles.insert([1; 32], record);
    bbg::certificate::StateCertificate::from_state(&state, &[0]).unwrap()
}

#[test]
fn successful_execution_consumes_exactly_the_ordered_active_stream() {
    let warrior = Warrior::new();
    let program = bundle("[3 [[16 [[1 0] [1 0]]] [16 [[1 0] [1 0]]]]]");
    let exact = input(&[], &[123456789, 987654321]);
    assert_eq!(warrior.run(&program, &exact).unwrap().output, exact.secret);
    assert!(warrior
        .verify_by_rerun(&program, &exact, &exact.secret)
        .unwrap());
    for secret in [
        vec![],
        vec![123456789],
        vec![123456789, 987654321, 777777777],
    ] {
        let error = warrior.run(&program, &input(&[], &secret)).unwrap_err();
        for value in &secret {
            assert!(
                !error.contains(&value.to_string()),
                "witness leaked in diagnostic"
            );
        }
    }
    let error = warrior
        .run(&bundle("[1 7]"), &input(&[], &[123456789]))
        .unwrap_err();
    assert_eq!(error, "unused secret inputs");
}

#[test]
fn inactive_calls_do_not_consume_witnesses_and_nested_checks_preserve_order() {
    let warrior = Warrior::new();
    let branch = bundle("[4 [[0 2] [[16 [[1 0] [1 0]]] [1 9]]]]");
    assert_eq!(
        warrior.run(&branch, &input(&[0], &[41])).unwrap().output,
        [41]
    );
    assert_eq!(warrior.run(&branch, &input(&[1], &[])).unwrap().output, [9]);
    assert_eq!(
        warrior.run(&branch, &input(&[1], &[41])).unwrap_err(),
        "unused secret inputs"
    );
    let nested = bundle("[16 [[1 0] [9 [[0 2] [16 [[1 0] [1 0]]]]]]]");
    assert_eq!(
        warrior.run(&nested, &input(&[], &[31, 31])).unwrap().output,
        [31]
    );
    assert!(warrior
        .run(&nested, &input(&[], &[31]))
        .unwrap_err()
        .contains("halted"));
    assert!(warrior
        .run(&nested, &input(&[], &[31, 32]))
        .unwrap_err()
        .contains("rejected"));
}

#[test]
fn state_execution_enforces_the_same_stream_contract_for_hidden_reads() {
    let warrior = Warrior::new();
    let state = state();
    let mut lookup = bundle("[17 [[1 0] [16 [[1 0] [1 0]]]]]");
    lookup.reads_state = true;
    let run =
        |secret: &[u64]| warrior.run_state_certificate(&lookup, &input(&[], secret), &state, 1000);
    assert_eq!(run(&[11]).unwrap().output, [77]);
    assert!(run(&[]).is_err());
    assert_eq!(run(&[11, 123456789]).unwrap_err(), "unused secret inputs");
    let mut branch = bundle("[4 [[1 1] [[17 [[1 0] [16 [[1 0] [1 0]]]]] [1 9]]]]");
    branch.reads_state = true;
    assert_eq!(
        warrior
            .run_state_certificate(&branch, &input(&[], &[]), &state, 1000)
            .unwrap()
            .output,
        [9]
    );
    assert_eq!(
        warrior
            .run_state_certificate(&branch, &input(&[], &[123456789]), &state, 1000)
            .unwrap_err(),
        "unused secret inputs"
    );
}

#[test]
fn canonical_field_inputs_are_required_in_both_native_entry_points() {
    let warrior = Warrior::new();
    let state = state();
    for secret in [false, true] {
        let program = bundle(if secret {
            "[16 [[1 0] [1 0]]]"
        } else {
            "[0 2]"
        });
        let values = |value| {
            if secret {
                input(&[], &[value])
            } else {
                input(&[value], &[])
            }
        };
        let valid = values(nebu::field::P - 1);
        assert_eq!(
            warrior.run(&program, &valid).unwrap().output,
            [nebu::field::P - 1]
        );
        assert_eq!(
            warrior
                .run_state_certificate(&program, &valid, &state, 1000)
                .unwrap()
                .output,
            [nebu::field::P - 1]
        );
        for bad in [nebu::field::P, u64::MAX] {
            let error = warrior.run(&program, &values(bad)).unwrap_err();
            assert!(error.contains("canonical"));
            assert!(!error.contains(&bad.to_string()));
            let error = warrior
                .run_state_certificate(&program, &values(bad), &state, 1000)
                .unwrap_err();
            assert!(!error.contains(&bad.to_string()));
        }
    }
}
