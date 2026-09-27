use joy_rs::Warrior;
use trident::runtime::{ProgramBundle, ProgramInput, Prover, Runner};
fn compiled() -> ProgramBundle {
    let source = "program private_product\nfn main(a: Field) -> Field { let x: Field = divine()\n let y: Field = divine()\n x * y + a }\n";
    let options = trident::CompileOptions::default()
        .with_package(joy_rs::target_package("nox").unwrap())
        .unwrap();
    let assembly = trident::compile_with_options(source, "private_product.tri", &options).unwrap();
    ProgramBundle {
        name: "private_product".into(),
        version: "1".into(),
        target_vm: "nox".into(),
        target_os: None,
        source_hash: trident::hash::content_hash_bytes(source.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        assembly,
        entry_point: "main".into(),
        functions: vec![],
        reads_state: false,
        cost: trident::runtime::artifact::BundleCost {
            table_values: vec![],
            table_names: vec![],
            padded_height: 0,
            estimated_proving_ns: 0,
        },
    }
}
#[test]
fn secret_execution_works_but_all_public_provers_refuse_witnesses() {
    let bundle = compiled();
    let input = ProgramInput {
        public: vec![3],
        secret: vec![7, 13],
        digests: vec![],
    };
    let warrior = Warrior::new();
    assert_eq!(warrior.run(&bundle, &input).unwrap().output, vec![94]);
    assert!(warrior.verify_by_rerun(&bundle, &input, &[94]).unwrap());
    assert!(warrior
        .prove(&bundle, &input)
        .unwrap_err()
        .contains("secret inputs"));
    assert!(warrior
        .prove_execution(&bundle, &input, 10000)
        .unwrap_err()
        .contains("secret inputs"));
    assert!(warrior
        .prove_zheng(&bundle, &input)
        .unwrap_err()
        .contains("secret inputs"));
    assert!(warrior
        .prove_zheng_with_state(&bundle, &input, &bbg::BbgState::new())
        .unwrap_err()
        .contains("secret inputs"));
    let state = bbg::BbgState::new();
    let certificate =
        bbg::certificate::StateCertificate::from_state(&state, &(0..10).collect::<Vec<_>>())
            .unwrap();
    assert!(warrior
        .prove_state_execution(&bundle, &input, &state, 10000)
        .unwrap_err()
        .contains("no secret"));
    assert!(warrior
        .prove_state_certificate(&bundle, &input, &certificate, 10000)
        .unwrap_err()
        .contains("no secret"));
}

#[test]
fn missing_witnesses_are_refused_by_execution() {
    let bundle = compiled();
    for secret in [vec![], vec![7]] {
        let input = ProgramInput {
            public: vec![3],
            secret,
            digests: vec![],
        };
        assert!(Warrior::new().run(&bundle, &input).is_err());
    }
}

#[test]
fn relabeling_legacy_statement_cannot_enable_another_format() {
    let mut bundle = compiled();
    bundle.assembly = "[5 [[1 3] [1 5]]]".into();
    let warrior = Warrior::new();
    let input = ProgramInput {
        public: vec![],
        secret: vec![],
        digests: vec![],
    };
    let (mut artifact, _) = warrior.prove_zheng(&bundle, &input).unwrap();
    for format in [
        "joy-nox-ccs-triton7-zk-v3",
        joy_rs::EXECUTION_FORMAT,
        joy_rs::STATE_EXECUTION_FORMAT,
    ] {
        artifact.format = format.into();
        assert!(warrior
            .verify_zheng(&bundle, &artifact)
            .unwrap_err()
            .contains("format"));
        assert!(warrior
            .verify_artifact(&artifact)
            .unwrap_err()
            .contains("format"));
    }
}
