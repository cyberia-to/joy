use joy_rs::{Warrior, ZkExecutionArtifact, EXECUTION_FORMAT, ZK_EXECUTION_FORMAT};
use std::sync::OnceLock;
use trident::runtime::{artifact::BundleCost, ProgramBundle, ProgramInput, Prover, Verifier};
use zheng::execution::{ExecutionNoun, ExecutionStatement};

fn bundle(assembly: &str) -> ProgramBundle {
    ProgramBundle {
        name: "native_private".into(),
        version: "1".into(),
        target_vm: "nox".into(),
        target_os: None,
        source_hash: "review-source-identity".into(),
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

fn fixture() -> &'static (ProgramBundle, ProgramInput, ZkExecutionArtifact) {
    static FIXTURE: OnceLock<(ProgramBundle, ProgramInput, ZkExecutionArtifact)> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let bundle = bundle("[5 [[7 [[16 [[1 0] [1 0]]] [16 [[1 0] [1 0]]]]] [0 2]]]");
        let input = input(&[3], &[7, 13]);
        // The explicit budget must override the warrior's configured budget.
        let (artifact, result) = Warrior::with_budget(1)
            .prove_zk_execution(&bundle, &input, 9)
            .unwrap();
        assert_eq!(result.output, [94]);
        assert_eq!(result.cycle_count, 9);
        artifact.verify().unwrap();
        (bundle, input, artifact)
    })
}

#[test]
fn native_profiles_select_correctly_and_private_proofs_use_fresh_randomness() {
    let (program, values, artifact) = fixture();
    let warrior = Warrior::with_budget(9);
    let private = warrior.prove(program, values).unwrap();
    assert_eq!(private.format, ZK_EXECUTION_FORMAT);
    assert!(warrior.verify(&private).unwrap());
    let fresh = ZkExecutionArtifact::from_bytes(&private.proof_bytes).unwrap();
    assert_eq!(fresh.statement, artifact.statement);
    assert_ne!(fresh.proof, artifact.proof);
    let public_bundle = bundle("[5 [[0 2] [1 7]]]");
    let public = warrior.prove(&public_bundle, &input(&[3], &[])).unwrap();
    assert_eq!(public.format, EXECUTION_FORMAT);
    assert!(warrior.verify(&public).unwrap());
    for original in [private, public] {
        for field in 0..4 {
            let mut forged = original.clone();
            match field {
                0 => forged.claim.public_input[0] += 1,
                1 => forged.claim.public_output[0] += 1,
                2 => forged.claim.program_hash[0] ^= 1,
                _ => forged.format = "unknown-proof-profile".into(),
            }
            assert!(
                !warrior.verify(&forged).unwrap_or(false),
                "accepted claim mutation {field}"
            );
        }
    }
}

#[test]
fn every_public_statement_and_build_identity_is_bound() {
    let (bundle, _, original) = fixture();
    assert!(original.matches_program(bundle).unwrap());
    for change in 0..11 {
        let mut forged = original.clone();
        match change {
            0 => forged.statement.execution.public_input[0] += 1,
            1 => forged.statement.execution.public_output[0] += 1,
            2 => forged.statement.execution.cycles -= 1,
            3 => forged.statement.execution.budget += 1,
            4 => forged.statement.execution.budget -= 1,
            5 => forged.source_hash.push('x'),
            6 => forged.program.push('x'),
            7 => forged.assembly = "[1 0]".into(),
            8 => {
                forged.assembly = "[1 0]".into();
                let program = ExecutionNoun::Pair(
                    Box::new(ExecutionNoun::Atom(1)),
                    Box::new(ExecutionNoun::Atom(0)),
                );
                forged.statement.execution.program =
                    ExecutionStatement::encode_program(&program).unwrap();
                forged.statement.execution.public_output = vec![0];
                forged.statement.execution.cycles = 1;
            }
            9 => forged.root_in_subject = true,
            _ => forged.format = EXECUTION_FORMAT.into(),
        }
        assert!(
            forged.verify().is_err(),
            "accepted statement mutation {change}"
        );
    }
    for change in 0..5 {
        let mut other = bundle.clone();
        match change {
            0 => other.target_vm = "another-vm".into(),
            1 => other.reads_state = true,
            2 => other.source_hash.push('x'),
            3 => other.name.push('x'),
            _ => other.assembly = "[1 0]".into(),
        }
        assert!(!original.matches_program(&other).unwrap());
    }
}

#[test]
fn explicit_budget_and_expected_claim_are_enforced_without_secret_replay() {
    let (bundle, input, artifact) = fixture();
    assert_eq!(artifact.statement.execution.budget, 9);
    assert!(artifact.claim_matches(Some(&[94]), Some(&[3]), 9));
    assert!(!artifact.claim_matches(Some(&[95]), Some(&[3]), 9));
    assert!(!artifact.claim_matches(Some(&[94]), Some(&[4]), 9));
    assert!(!artifact.claim_matches(Some(&[94]), Some(&[3]), 8));
    assert!(Warrior::new().prove_zk_execution(bundle, input, 8).is_err());
    for secret in [vec![], vec![7], vec![7, 13, 29]] {
        let mut bad = input.clone();
        bad.secret = secret;
        assert!(Warrior::new().prove_zk_execution(bundle, &bad, 9).is_err());
    }
}

fn unchecked_bytes(artifact: &ZkExecutionArtifact) -> Vec<u8> {
    let mut bytes = b"JOYZH001".to_vec();
    bytes.extend(postcard::to_allocvec(artifact).unwrap());
    bytes
}

#[test]
fn malformed_outer_metadata_and_inner_wire_fail_at_their_boundaries() {
    let (_, _, original) = fixture();
    let bytes = original.to_bytes().unwrap();
    ZkExecutionArtifact::from_bytes(&bytes)
        .unwrap()
        .verify()
        .unwrap();
    for length in [0, 7, 8, 20, bytes.len() - 1] {
        assert!(ZkExecutionArtifact::from_bytes(&bytes[..length]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(ZkExecutionArtifact::from_bytes(&trailing).is_err());
    let mut retired = bytes;
    retired[..8].copy_from_slice(b"JOYZK003");
    assert!(ZkExecutionArtifact::from_bytes(&retired).is_err());
    for change in 0..6 {
        let mut malformed = original.clone();
        match change {
            0 => malformed.source_hash.clear(),
            1 => malformed.source_hash = "x".repeat(129),
            2 => malformed.program = "x".repeat(4097),
            3 => malformed.assembly = " ".repeat(256 * 1024 + 1),
            4 => malformed.format = "x".repeat(129),
            _ => malformed.proof.clear(),
        }
        assert!(ZkExecutionArtifact::from_bytes(&unchecked_bytes(&malformed)).is_err());
    }
    // This prefix ends exactly before the proof vector's postcard length.
    let mut prefix = b"JOYZH001".to_vec();
    prefix.extend(
        postcard::to_allocvec(&(
            &original.format,
            &original.program,
            &original.assembly,
            &original.source_hash,
            &original.statement,
        ))
        .unwrap(),
    );
    for length in [256 * 1024 * 1024usize, 256 * 1024 * 1024 + 1, usize::MAX] {
        let mut bomb = prefix.clone();
        bomb.extend(postcard::to_allocvec(&length).unwrap());
        assert!(ZkExecutionArtifact::from_bytes(&bomb).is_err());
    }
    for change in 0..5 {
        let mut malformed = original.clone();
        match change {
            0 => malformed.proof[0] ^= 1,
            1 => malformed.proof[8] ^= 1,
            2 => {
                malformed.proof.pop();
            }
            3 => malformed.proof.push(0),
            _ => malformed.proof[53..61].copy_from_slice(&nebu::field::P.to_le_bytes()),
        }
        let decoded = ZkExecutionArtifact::from_bytes(&unchecked_bytes(&malformed)).unwrap();
        assert!(
            decoded.verify().is_err(),
            "accepted inner proof mutation {change}"
        );
    }
}

#[test]
fn hidden_namespace_uses_authenticated_tables_and_canonical_certificate_metadata() {
    let state = bbg::BbgState::new();
    let certificate =
        bbg::certificate::StateCertificate::from_state(&state, &(0..10).collect::<Vec<_>>())
            .unwrap();
    let mut bundle = bundle("[17 [[16 [[1 0] [1 0]]] [16 [[1 0] [1 0]]]]]");
    bundle.reads_state = true;
    let input = input(&[], &[9, 0]);
    let (artifact, result) = Warrior::new()
        .prove_zk_state_certificate(&bundle, &input, &certificate, 7)
        .unwrap();
    assert_eq!(result.output, [certificate.dimensions[9].fields[0]]);
    assert_eq!(result.cycle_count, 7);
    artifact.verify().unwrap();
    for change in 0..6 {
        let mut forged = artifact.clone();
        let state = forged.state.as_mut().unwrap();
        match change {
            0 => state.dimensions.swap(0, 9),
            1 => state.dimensions[9].namespace = 8,
            2 => {
                state.dimensions.pop();
            }
            3 => state.version += 1,
            4 => state.lens_version += 1,
            _ => state.dimensions[9].fields[0] += 1,
        }
        assert!(
            forged.verify().is_err(),
            "accepted certificate mutation {change}"
        );
    }
}
