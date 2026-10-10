//! End-to-end tests: ProgramBundle in, executed output out.
//!
//! The .bundle.json fixture is hand-built against
//! trident/src/runtime/artifact.rs (trident's CLI has no bundle-emit
//! command yet); bundle generation itself is exercised for real through
//! the trident API in `compile_tri_end_to_end`.

use std::path::PathBuf;

use joy_rs::Warrior;
use trident::runtime::artifact::BundleCost;
use trident::runtime::{ProgramBundle, ProgramInput, Prover, Runner, Verifier};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn bundle(assembly: &str) -> ProgramBundle {
    ProgramBundle {
        name: "test".to_string(),
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

fn input(public: &[u64], secret: &[u64]) -> ProgramInput {
    ProgramInput {
        public: public.to_vec(),
        secret: secret.to_vec(),
        digests: Vec::new(),
    }
}

#[test]
fn add_literals_reduces_to_eight() {
    let warrior = Warrior::new();
    let result = warrior
        .run(&bundle("[5 [[1 3] [1 5]]]"), &input(&[], &[]))
        .expect("run failed");
    assert_eq!(result.output, vec![8]);
    // One trace row per budget unit: add(1) + quote(1) + quote(1).
    assert_eq!(result.cycle_count, 3);
}

#[test]
fn public_inputs_bind_as_subject() {
    // trident-emitted formula for:
    //   fn main(a: Field, b: Field) -> Field { let c = a + b; c * a }
    let formula = "[2 [[3 [[5 [[0 6] [0 2]]] [0 1]]] [1 [7 [[0 2] [0 14]]]]]]";
    let warrior = Warrior::new();
    let result = warrior
        .run(&bundle(formula), &input(&[3, 5], &[]))
        .expect("run failed");
    // (3 + 5) * 3 = 24
    assert_eq!(result.output, vec![24]);
}

#[test]
fn secret_input_served_by_call_pattern() {
    // [16 [[1 0] [1 0]]]: tag = quote 0, check = quote 0 (always accepts).
    let warrior = Warrior::new();
    let result = warrior
        .run(&bundle("[16 [[1 0] [1 0]]]"), &input(&[], &[42]))
        .expect("run failed");
    assert_eq!(result.output, vec![42]);
}

#[test]
fn missing_secret_halts_honestly() {
    let warrior = Warrior::new();
    let err = warrior
        .run(&bundle("[16 [[1 0] [1 0]]]"), &input(&[], &[]))
        .expect_err("must halt without witness");
    assert!(err.contains("halted"), "unexpected error: {}", err);
}

#[test]
fn budget_exhaustion_halts() {
    let warrior = Warrior::with_budget(1);
    let err = warrior
        .run(&bundle("[5 [[1 3] [1 5]]]"), &input(&[], &[]))
        .expect_err("must halt on budget");
    assert!(err.contains("halted"), "unexpected error: {}", err);
}

#[test]
fn wrong_target_vm_rejected() {
    let mut b = bundle("[1 0]");
    b.target_vm = "triton".to_string();
    let err = Warrior::new()
        .run(&b, &input(&[], &[]))
        .expect_err("must reject non-nox bundle");
    assert!(err.contains("triton"), "unexpected error: {}", err);
}

#[test]
fn fixture_bundle_json_runs() {
    let text = std::fs::read_to_string(fixture("add.bundle.json")).expect("fixture missing");
    let b = ProgramBundle::from_json(&text).expect("bundle parse failed");
    assert_eq!(b.target_vm, "nox");
    let result = Warrior::new()
        .run(&b, &input(&[], &[]))
        .expect("run failed");
    assert_eq!(result.output, vec![8]);
}

#[test]
fn verify_by_rerun_pass_and_fail() {
    let warrior = Warrior::new();
    let b = bundle("[5 [[1 3] [1 5]]]");
    assert!(warrior
        .verify_by_rerun(&b, &input(&[], &[]), &[8])
        .expect("verify failed"));
    assert!(!warrior
        .verify_by_rerun(&b, &input(&[], &[]), &[9])
        .expect("verify failed"));
}

#[test]
fn compile_tri_end_to_end() {
    // Real bundle generation through the trident API, then execution.
    let mut options = trident::CompileOptions::for_profile("debug");
    options.target_config = joy_rs::nox_terrain();
    let b = trident::compile_to_bundle(&fixture("add.tri"), &options).expect("compile failed");
    assert_eq!(b.target_vm, "nox");
    let result = Warrior::new()
        .run(&b, &input(&[3, 5], &[]))
        .expect("run failed");
    assert_eq!(result.output, vec![24]);
    assert!(result.cycle_count > 0);
}

#[test]
fn prove_works_on_a_minimal_bundle() {
    // The former honest dash: prove is real now (soft3 M4). The minimal
    // provable program has two trace rows (zheng folds row transitions);
    // add-literals gives three.
    let warrior = Warrior::new();
    let pd = warrior
        .prove(&bundle("[5 [[1 3] [1 5]]]"), &input(&[], &[]))
        .expect("prove failed");
    assert_eq!(pd.format, joy_rs::EXECUTION_FORMAT);
    assert!(warrior
        .verify(&pd)
        .expect("public execution claim verification"));
}

#[test]
fn verify_refuses_foreign_proof_formats() {
    let proof = trident::runtime::ProofData {
        claim: trident::field::proof::Claim {
            program_hash: Vec::new(),
            public_input: Vec::new(),
            public_output: Vec::new(),
        },
        proof_bytes: Vec::new(),
        format: "stark-triton-v2".to_string(),
    };
    let err = Warrior::new()
        .verify(&proof)
        .expect_err("foreign formats are refused, not guessed at");
    assert!(err.contains("format"), "unexpected error: {}", err);
}

// ── public execution proofs (profile v3) ─────────────────────────────────────

/// Compile the add.tri fixture through the real trident API.
fn compiled_add() -> ProgramBundle {
    let mut options = trident::CompileOptions::for_profile("debug");
    options.target_config = joy_rs::nox_terrain();
    trident::compile_to_bundle(&fixture("add.tri"), &options).expect("compile failed")
}

#[test]
fn prove_verify_roundtrip_through_traits_and_disk() {
    let warrior = Warrior::new();
    let b = compiled_add();
    let pd = warrior
        .prove(&b, &input(&[3, 5], &[]))
        .expect("trait prove failed");
    assert_eq!(pd.format, joy_rs::EXECUTION_FORMAT);
    assert_eq!(pd.claim.public_output, vec![24]);
    assert!(warrior
        .verify(&pd)
        .expect("public execution claim verification"));

    // Disk round-trip: save, load, verify.
    let dir = std::env::temp_dir().join("joy-proof-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("add.zheng.json");
    let artifact = joy_rs::ExecutionArtifact::from_bytes(&pd.proof_bytes).unwrap();
    artifact.save(&path).expect("save failed");
    let loaded = joy_rs::ExecutionArtifact::load(&path).expect("load failed");
    loaded.verify().expect("execution verification failed");
}

/// Compile the hash.tri fixture (trident's hash builtin -> pattern 15).
fn compiled_hash() -> ProgramBundle {
    let mut options = trident::CompileOptions::for_profile("debug");
    options.target_config = joy_rs::nox_terrain();
    trident::compile_to_bundle(&fixture("hash.tri"), &options).expect("compile failed")
}

#[test]
fn proof_for_a_different_program_rejected() {
    let warrior = Warrior::new();
    let pd = warrior
        .prove(&compiled_add(), &input(&[3, 5], &[]))
        .unwrap();
    let artifact = joy_rs::ExecutionArtifact::from_bytes(&pd.proof_bytes).unwrap();
    assert!(artifact.matches_program(&compiled_add().assembly).unwrap());
    assert!(!artifact.matches_program("[5 [[1 3] [1 5]]]").unwrap());
}

#[test]
fn output_claim_is_bound_by_the_certificate() {
    let b = compiled_add();
    let warrior = Warrior::new();
    let mut pd = warrior.prove(&b, &input(&[3, 5], &[])).unwrap();
    assert_eq!(pd.claim.public_output, vec![24]);
    pd.claim.public_output = vec![999];
    assert!(!warrior.verify(&pd).unwrap());
}

#[test]
fn hash_program_proves_and_verifies() {
    let warrior = Warrior::new();
    let b = compiled_hash();
    let pd = warrior
        .prove(&b, &input(&[42], &[]))
        .expect("hash prove failed");
    assert_eq!(pd.format, joy_rs::EXECUTION_FORMAT);
    assert_eq!(
        pd.claim.public_output.len(),
        4,
        "hash returns a 4-limb digest"
    );
    assert_eq!(
        pd.claim.public_output,
        warrior.run(&b, &input(&[42], &[])).unwrap().output
    );
    assert!(warrior.verify(&pd).unwrap(), "hash proof must verify");
    for limb in 0..4 {
        let mut bad = pd.clone();
        bad.claim.public_output[limb] ^= 1;
        assert!(!warrior.verify(&bad).unwrap(), "digest limb {limb}");
    }
}

// State acceptance uses the authenticated execution protocol.
fn look_assembly(root: &[u8; 32], ns: u64, key: u64) -> String {
    let l: Vec<_> = root
        .chunks_exact(8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let (l0, l1, l2, l3) = (l[0], l[1], l[2], l[3]);
    format!("[2 [[3 [[3 [[1 {l0}] [3 [[1 {l1}] [3 [[1 {l2}] [1 {l3}]]]]]]] [1 0]]] [1 [17 [[1 {ns}] [1 {key}]]]]]]")
}
fn sample_state() -> bbg::BbgState {
    let mut state = bbg::BbgState::new();
    for (key, energy) in [(1, 77), (2, 88)] {
        let mut record = bbg::types::ParticleRecord::zero();
        record.energy = energy;
        state.particles.insert([key; 32], record);
    }
    state
}
#[test]
fn look_program_proves_and_verifies_against_state() {
    let state = sample_state();
    let root = state.root();
    let b = bundle(&look_assembly(&root, 0, 11));
    let warrior = Warrior::new();
    let (artifact, result) = warrior
        .prove_state_execution(&b, &input(&[], &[]), &state, 1000)
        .unwrap();
    assert_eq!(result.output, vec![77]);
    assert_eq!(
        artifact
            .statement
            .state_root
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
        root
    );
    assert!(artifact.matches_program(&b).unwrap());
    artifact.verify().unwrap();
    let decoded =
        joy_rs::StateExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
    decoded.verify().unwrap();
    assert!(warrior.verify(&decoded.proof_data().unwrap()).unwrap());
}
#[test]
fn look_against_stale_root_refused_at_prove() {
    let mut state = sample_state();
    let old = state.root();
    state.particles.get_mut(&[1; 32]).unwrap().energy = 99;
    let b = bundle(&look_assembly(&old, 0, 11));
    assert!(Warrior::new()
        .prove_state_execution(&b, &input(&[], &[]), &state, 1000)
        .is_err());
    state.refresh_root();
    assert!(Warrior::new()
        .prove_state_execution(&b, &input(&[], &[]), &state, 1000)
        .is_err());
}
#[test]
fn look_artifact_root_mismatch_rejected() {
    let state = sample_state();
    let b = bundle(&look_assembly(&state.root(), 0, 11));
    let (artifact, _) = Warrior::new()
        .prove_state_execution(&b, &input(&[], &[]), &state, 1000)
        .unwrap();
    for i in 0..4 {
        let mut bad = artifact.clone();
        bad.statement.state_root[i] ^= 1;
        assert!(bad.verify().is_err());
    }
}
#[test]
fn look_artifact_tampered_binding_and_table_rejected() {
    let state = sample_state();
    let b = bundle(&look_assembly(&state.root(), 0, 11));
    let (artifact, _) = Warrior::new()
        .prove_state_execution(&b, &input(&[], &[]), &state, 1000)
        .unwrap();
    for which in 0..6 {
        let mut bad = artifact.clone();
        match which {
            0 => bad.statement.reads[0].namespace = 1,
            1 => bad.statement.reads[0].key += 1,
            2 => bad.statement.reads[0].value += 1,
            3 => bad.statement.execution.public_output[0] += 1,
            4 => bad.certificate.dimensions[0].fields[11] += 1,
            _ => bad.statement.reads.clear(),
        }
        assert!(bad.verify().is_err(), "mutation {which}");
    }
}
