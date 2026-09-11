//! Public execution artifacts bind actual program, inputs, outputs and cost.
use joy_rs::{ExecutionArtifact, Warrior, EXECUTION_FORMAT};
use trident::runtime::{ProgramBundle, ProgramInput, Prover, Verifier};
fn bundle(assembly: &str) -> ProgramBundle {
    ProgramBundle {
        name: "public-execution-test".into(),
        version: "0.4.0".into(),
        target_vm: "nox".into(),
        target_os: None,
        assembly: assembly.into(),
        entry_point: "main".into(),
        functions: vec![],
        cost: trident::runtime::artifact::BundleCost {
            table_values: vec![],
            table_names: vec![],
            padded_height: 0,
            estimated_proving_ns: 0,
        },
        source_hash: String::new(),
        reads_state: false,
    }
}
fn input(public: &[u64]) -> ProgramInput {
    ProgramInput {
        public: public.to_vec(),
        secret: vec![],
        digests: vec![],
    }
}
#[test]
fn compiled_source_roundtrip_and_changed_statement_rejection() {
    let source="program execution\nfn main(a: Field, b: Field) -> Field { let c: Field = a + b\n c * a }\n";
    let mut options = trident::CompileOptions::default();
    options.target_config = trident::target::TerrainConfig::resolve("nox").unwrap();
    let assembly = trident::compile_with_options(source, "execution.tri", &options).unwrap();
    let warrior = Warrior::new();
    let (artifact, native) = warrior
        .prove_execution(&bundle(&assembly), &input(&[3, 5]), 10000)
        .unwrap();
    assert_eq!(native.output, vec![24]);
    assert_eq!(artifact.statement.public_output, native.output);
    assert_eq!(artifact.statement.cycles, native.cycle_count);
    let decoded = ExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
    decoded.verify().unwrap();
    assert!(decoded.matches_program(&assembly).unwrap());
    assert!(!decoded.matches_program("[1 24]").unwrap());
    for mutation in 0..5 {
        let mut bad = decoded.clone();
        match mutation {
            0 => bad.statement.public_output[0] += 1,
            1 => bad.statement.public_input[0] += 1,
            2 => bad.statement.cycles += 1,
            3 => bad.statement.budget += 1,
            _ => bad.assembly = "[1 24]".into(),
        }
        assert!(bad.verify().is_err(), "mutation {mutation} accepted");
    }
}
#[test]
fn prover_and_verifier_traits_bind_claim_without_legacy_downgrade() {
    let warrior = Warrior::new();
    let b = bundle("[5 [[0 2] [1 7]]]");
    let proof = warrior.prove(&b, &input(&[5])).unwrap();
    assert_eq!(proof.format, EXECUTION_FORMAT);
    assert_eq!(proof.claim.public_output, vec![12]);
    assert!(warrior.verify(&proof).unwrap());
    let mut bad = proof.clone();
    bad.claim.public_output[0] = 99;
    assert!(!warrior.verify(&bad).unwrap());
    let mut bad = proof.clone();
    bad.claim.public_input[0] = 99;
    assert!(!warrior.verify(&bad).unwrap());
    let mut bad = proof.clone();
    bad.claim.program_hash[0] ^= 1;
    assert!(!warrior.verify(&bad).unwrap());
    let mut bad = proof;
    bad.format = joy_rs::PROOF_FORMAT.into();
    assert!(warrior.verify(&bad).is_err());
}
#[test]
fn unsupported_secret_state_and_call_inputs_are_refused() {
    let warrior = Warrior::new();
    let b = bundle("[1 7]");
    let mut secret = input(&[]);
    secret.secret = vec![0x123456789];
    let error = warrior.prove_execution(&b, &secret, 1000).unwrap_err();
    assert!(error.contains("secret"));
    assert!(!error.contains("4886718345"));
    let mut state = b.clone();
    state.reads_state = true;
    assert!(warrior.prove_execution(&state, &input(&[]), 1000).is_err());
    assert!(warrior
        .prove_execution(&bundle("[16 [[1 0] [1 0]]]"), &input(&[]), 1000)
        .is_err());
}
#[test]
fn artifact_format_and_claim_filters_are_exact() {
    let (artifact, _) = Warrior::new()
        .prove_execution(&bundle("[1 7]"), &input(&[3]), 100)
        .unwrap();
    assert!(artifact.claim_matches(Some(&[7]), Some(&[3]), 100));
    assert!(!artifact.claim_matches(Some(&[8]), Some(&[3]), 100));
    assert!(!artifact.claim_matches(Some(&[7]), Some(&[4]), 100));
    assert!(!artifact.claim_matches(Some(&[7]), Some(&[3]), 99));
    let mut bytes = artifact.to_bytes().unwrap();
    bytes.push(0);
    assert!(ExecutionArtifact::from_bytes(&bytes).is_err());
    let mut bad = artifact.clone();
    bad.format = "other".into();
    assert!(ExecutionArtifact::from_bytes(&bad.to_bytes().unwrap()).is_err());
    let mut bad = artifact;
    bad.proof.spartan.eval_value += nebu::Goldilocks::ONE;
    assert!(bad.verify().is_err());
}
