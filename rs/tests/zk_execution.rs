use joy_rs::{Warrior, ZkExecutionArtifact};
use trident::runtime::{ProgramBundle, ProgramInput};
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
fn compiled_private_execution_real_zk_roundtrip_and_claim_mutations() {
    let bundle = compiled();
    let input = ProgramInput {
        public: vec![3],
        secret: vec![7, 13],
        digests: vec![],
    };
    let started = std::time::Instant::now();
    let (artifact, native) = Warrior::new()
        .prove_zk_execution(&bundle, &input, 10000)
        .unwrap();
    assert_eq!(native.output, vec![94]);
    let wire = artifact.to_bytes().unwrap();
    assert_eq!(&wire[..8], b"JOYZK003");
    assert_eq!(trisha_rs::ccs::FORMAT, "zheng-ccs-triton7-zk-v2");
    for old_header in [b"JOYZK001", b"JOYZK002"] {
        let mut old = wire.clone();
        old[..8].copy_from_slice(old_header);
        assert!(ZkExecutionArtifact::from_bytes(&old).is_err());
    }
    eprintln!(
        "Joy private execution: {} bytes, prove {:?}",
        wire.len(),
        started.elapsed()
    );
    let decoded = ZkExecutionArtifact::from_bytes(&wire).unwrap();
    decoded.verify().unwrap();
    // Public metadata contains the expected IO only. Witness is never a wire field.
    assert_eq!(decoded.statement.execution.public_input, vec![3]);
    assert_eq!(decoded.statement.execution.public_output, vec![94]);
    let envelope = trisha_rs::ccs::decode_proof(&decoded.proof).unwrap();
    assert_eq!(envelope.claim.public_output, Vec::<u64>::new());
    let proof_data = decoded.proof_data().unwrap();
    assert!(joy_rs::zk_execution::verify_proof_data(&proof_data).unwrap());
    for mutation in 0..8 {
        let mut bad = decoded.clone();
        match mutation {
            0 => bad.statement.execution.public_input[0] += 1,
            1 => bad.statement.execution.public_output[0] += 1,
            2 => bad.statement.execution.cycles += 1,
            3 => bad.statement.execution.budget += 1,
            4 => bad.assembly = "[1 94]".into(),
            5 => bad.source_hash.push('0'),
            6 => bad.program.push('x'),
            _ => bad.format = "joy-nox-ccs-triton-zk-v2".into(),
        }
        assert!(bad.verify().is_err(), "accepted mutation {mutation}");
    }
    let mut trailing = wire;
    trailing.push(0);
    assert!(ZkExecutionArtifact::from_bytes(&trailing).is_err());
    assert!(joy_rs::ExecutionArtifact::from_bytes(&trailing).is_err());
}
#[test]
fn missing_and_excess_secret_streams_fail_before_proving() {
    let bundle = compiled();
    for secret in [vec![], vec![7], vec![7, 13, 19]] {
        let input = ProgramInput {
            public: vec![3],
            secret,
            digests: vec![],
        };
        assert!(Warrior::new()
            .prove_zk_execution(&bundle, &input, 10000)
            .is_err());
    }
}

#[test]
fn runtime_traits_route_secrets_to_private_proofs_and_reject_forged_claims() {
    use trident::runtime::{Prover, Verifier};
    let mut bundle = compiled();
    bundle.assembly = "[16 [[1 0] [1 0]]]".into();
    bundle.source_hash = trident::hash::content_hash_bytes(bundle.assembly.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let input = ProgramInput {
        public: vec![],
        secret: vec![42],
        digests: vec![],
    };
    let warrior = Warrior::new();
    let mut proof = warrior.prove(&bundle, &input).unwrap();
    assert_eq!(proof.format, joy_rs::ZK_EXECUTION_FORMAT);
    assert!(warrior.verify(&proof).unwrap());
    proof.claim.public_output[0] = 43;
    assert!(!warrior.verify(&proof).unwrap());
}
