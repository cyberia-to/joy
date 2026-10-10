//! `JOYST002` carries zheng's state-public envelope (profile 3, certificate
//! v3); retired `JOYST001` artifacts (v1 direct proofs) still verify.
use joy_rs::state_execution::{StateProof, LEGACY_STATE_EXECUTION_FORMAT};
use joy_rs::{StateExecutionArtifact, Warrior, STATE_EXECUTION_FORMAT};
use trident::runtime::artifact::BundleCost;
use trident::runtime::{ProgramBundle, ProgramInput};

fn bundle(assembly: &str) -> ProgramBundle {
    ProgramBundle {
        name: "state".to_string(),
        version: "0.1.0".to_string(),
        target_vm: "nox".to_string(),
        target_os: None,
        assembly: assembly.to_string(),
        entry_point: "main".to_string(),
        functions: Vec::new(),
        cost: BundleCost {
            table_values: Vec::new(),
            table_names: Vec::new(),
            padded_height: 0,
            estimated_proving_ns: 0,
        },
        source_hash: trident::hash::ContentHash(trident::hash::content_hash_bytes(
            assembly.as_bytes(),
        ))
        .to_hex(),
        reads_state: false,
    }
}

fn input() -> ProgramInput {
    ProgramInput {
        public: vec![],
        secret: vec![],
        digests: vec![],
    }
}

/// A raw formula that builds the root itself and reads (0, 11).
fn look_assembly(root: &[u8; 32]) -> String {
    let l: Vec<_> = root
        .chunks_exact(8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let (l0, l1, l2, l3) = (l[0], l[1], l[2], l[3]);
    format!("[2 [[3 [[3 [[1 {l0}] [3 [[1 {l1}] [3 [[1 {l2}] [1 {l3}]]]]]]] [1 0]]] [1 [17 [[1 0] [1 11]]]]]]")
}

fn state() -> bbg::BbgState {
    let mut state = bbg::BbgState::new();
    let mut record = bbg::types::ParticleRecord::zero();
    record.energy = 77;
    state.particles.insert([1; 32], record);
    state
}

fn proved() -> (StateExecutionArtifact, ProgramBundle, bbg::BbgState) {
    let state = state();
    let b = bundle(&look_assembly(&state.root()));
    let (artifact, result) = Warrior::new()
        .prove_state_execution(&b, &input(), &state, 1000)
        .unwrap();
    assert_eq!(result.output, vec![77]);
    (artifact, b, state)
}

#[test]
fn produced_state_artifacts_carry_a_state_public_envelope() {
    let (artifact, _, _) = proved();
    assert_eq!(artifact.format, STATE_EXECUTION_FORMAT);
    assert!(matches!(artifact.proof, StateProof::Certificate(_)));
    let bytes = artifact.to_bytes().unwrap();
    assert_eq!(&bytes[..8], b"JOYST002");
    let at = bytes
        .windows(8)
        .position(|w| w == b"ZHENGPF1")
        .expect("the zheng envelope travels inside");
    assert_eq!(bytes[at + 10], 3, "profile byte: state-public");
    let decoded = StateExecutionArtifact::from_bytes(&bytes).unwrap();
    decoded.verify().unwrap();
    assert_eq!(decoded.to_bytes().unwrap(), bytes);
    // the envelope's profile byte is checked: a public profile is refused
    let mut swapped = bytes.clone();
    swapped[at + 10] = 0;
    assert!(StateExecutionArtifact::from_bytes(&swapped).is_err());
}

#[test]
fn every_state_claim_and_the_certificate_are_bound() {
    let (artifact, _, _) = proved();
    let StateProof::Certificate(certificate) = &artifact.proof else {
        unreachable!()
    };
    for i in 0..4 {
        let mut bad = artifact.clone();
        bad.statement.state_root[i] ^= 1;
        assert!(bad.verify().is_err(), "root limb {i}");
    }
    let mut bad = artifact.clone();
    bad.statement.reads[0].value += 1;
    assert!(bad.verify().is_err(), "read value");
    let mut bad = artifact.clone();
    bad.statement.execution.public_output[0] += 1;
    assert!(bad.verify().is_err(), "output");
    // program name and source hash are metadata (no proof binds a label of a disclosed
    // witness); the assembly is bound: it is the statement's program
    let mut renamed = artifact.clone();
    renamed.program.push('x');
    renamed.source_hash.push('0');
    renamed.verify().unwrap();
    let mut bad = artifact.clone();
    bad.assembly = bad.assembly.replacen('1', "2", 1);
    assert!(bad.verify().is_err(), "assembly");
    let mut longer = certificate.clone();
    longer.free.push(1);
    let mut bad = artifact.clone();
    bad.proof = StateProof::Certificate(longer);
    assert!(bad.verify().is_err(), "certificate");
}

#[test]
fn retired_v1_state_artifacts_still_verify() {
    let (_, b, state) = proved();
    let certificate =
        bbg::certificate::StateCertificate::from_state(&state, &(0..10).collect::<Vec<_>>())
            .unwrap();
    // the frozen v1 identity binding: hemera(format, |name|, name, |hash|, hash)
    let mut h = hemera::Hasher::new();
    h.update(LEGACY_STATE_EXECUTION_FORMAT.as_bytes());
    for s in [b.name.as_str(), b.source_hash.as_str()] {
        h.update(&(s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    }
    let context = *h.finalize().as_bytes();
    let program = joy_rs::execution::parse_program(&b.assembly).unwrap();
    let (statement, proof) = zheng::execution::state::prove_state_execution(
        &program,
        &[],
        1000,
        false,
        &context,
        &certificate.evidence().unwrap(),
    )
    .unwrap();
    let v1 = StateExecutionArtifact {
        format: LEGACY_STATE_EXECUTION_FORMAT.into(),
        program: b.name.clone(),
        assembly: b.assembly.clone(),
        source_hash: b.source_hash.clone(),
        statement,
        certificate,
        proof: StateProof::Direct(proof),
    };
    let bytes = v1.to_bytes().unwrap();
    assert_eq!(&bytes[..8], b"JOYST001");
    let decoded = StateExecutionArtifact::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded.proof, StateProof::Direct(_)));
    decoded.verify().unwrap();
    // a v1 proof cannot be relabelled as v3
    let mut relabelled = decoded.clone();
    relabelled.format = STATE_EXECUTION_FORMAT.into();
    assert!(relabelled.verify().is_err());
}
