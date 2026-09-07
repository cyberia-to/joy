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
        source_hash: String::new(),
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
    let result = Warrior::new().run(&b, &input(&[], &[])).expect("run failed");
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
    assert_eq!(pd.format, joy_rs::PROOF_FORMAT);
    assert!(warrior.verify(&pd).expect("verify errored"));
}

#[test]
fn prove_refuses_single_row_traces_honestly() {
    // One trace row = no transition to fold. Honest refusal, not a fake.
    let err = Warrior::new()
        .prove_zheng(&bundle("[1 5]"), &input(&[], &[]))
        .expect_err("single-row trace must be refused");
    assert!(err.contains("row"), "unexpected error: {}", err);
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


// ── zheng prove / verify ─────────────────────────────────────────────────────

/// Compile the add.tri fixture through the real trident API.
fn compiled_add() -> ProgramBundle {
    let mut options = trident::CompileOptions::for_profile("debug");
    options.target_config = joy_rs::nox_terrain();
    trident::compile_to_bundle(&fixture("add.tri"), &options).expect("compile failed")
}

#[test]
fn prove_verify_roundtrip() {
    let warrior = Warrior::new();
    let b = compiled_add();
    let (artifact, result) = warrior
        .prove_zheng(&b, &input(&[3, 5], &[]))
        .expect("prove failed");
    assert_eq!(result.output, vec![24]);
    assert_eq!(artifact.meta.output, vec![24]);
    assert_eq!(artifact.format, joy_rs::PROOF_FORMAT);
    assert!(
        warrior.verify_zheng(&b, &artifact).expect("verify errored"),
        "honest proof must verify"
    );
}

#[test]
fn prove_verify_roundtrip_through_traits_and_disk() {
    let warrior = Warrior::new();
    let b = compiled_add();
    let pd = warrior
        .prove(&b, &input(&[3, 5], &[]))
        .expect("trait prove failed");
    assert_eq!(pd.format, joy_rs::PROOF_FORMAT);
    assert_eq!(pd.claim.public_output, vec![24]);
    assert!(warrior.verify(&pd).expect("trait verify errored"));

    // Disk round-trip: save, load, verify.
    let dir = std::env::temp_dir().join("joy-proof-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("add.zheng.json");
    let artifact: joy_rs::ProofArtifact = serde_json::from_slice(&pd.proof_bytes).unwrap();
    artifact.save(&path).expect("save failed");
    let loaded = joy_rs::ProofArtifact::load(&path).expect("load failed");
    assert!(warrior.verify_zheng(&b, &loaded).expect("verify errored"));
}

#[test]
fn tampered_proof_rejected() {
    let warrior = Warrior::new();
    let b = compiled_add();
    let (artifact, _) = warrior
        .prove_zheng(&b, &input(&[3, 5], &[]))
        .expect("prove failed");

    // Tamper a field inside the proof via the wire form.
    let mut v: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&artifact).unwrap()).unwrap();
    let ev = v["proof"]["groups"][0][0]["eval_value"].as_u64().unwrap();
    v["proof"]["groups"][0][0]["eval_value"] = serde_json::Value::from(ev ^ 1);
    let tampered: joy_rs::ProofArtifact = serde_json::from_value(v).unwrap();
    assert!(
        !warrior.verify_zheng(&b, &tampered).expect("verify errored"),
        "tampered proof must be rejected"
    );
}

#[test]
fn proof_for_a_different_program_rejected() {
    let warrior = Warrior::new();
    let b = compiled_add();
    let (artifact, _) = warrior
        .prove_zheng(&b, &input(&[3, 5], &[]))
        .expect("prove failed");
    // A different bundle: raw add-literals formula.
    let other = bundle("[5 [[1 3] [1 5]]]");
    assert!(
        !warrior.verify_zheng(&other, &artifact).expect("verify errored"),
        "proof must bind to its program"
    );
}

#[test]
fn artifact_load_error_paths() {
    // Missing file.
    let missing = joy_rs::ProofArtifact::load(std::path::Path::new("/nonexistent/p.zheng.json"));
    assert!(missing.is_err());

    // Malformed JSON.
    let dir = std::env::temp_dir().join("joy-proof-test");
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.zheng.json");
    std::fs::write(&bad, b"{not json").unwrap();
    assert!(joy_rs::ProofArtifact::load(&bad).is_err());

    // Unknown format string.
    let warrior = Warrior::new();
    let b = compiled_add();
    let (mut artifact, _) = warrior
        .prove_zheng(&b, &input(&[3, 5], &[]))
        .expect("prove failed");
    artifact.format = "not-zheng".to_string();
    let p = dir.join("wrongformat.zheng.json");
    artifact.save(&p).unwrap();
    let err = joy_rs::ProofArtifact::load(&p);
    assert!(err.is_err(), "unknown format must be refused");
}

#[test]
fn prove_refuses_look_rows_honestly() {
    // A look formula (tag 17) has no bbg state wired: prove must refuse,
    // not fabricate. NullCalls-style provider gives no look values, so the
    // reduction itself errors — either way, no proof comes out.
    let warrior = Warrior::new();
    let b = bundle("[17 [[1 0] [1 2]]]");
    let r = warrior.prove_zheng(&b, &input(&[], &[]));
    assert!(r.is_err(), "no proof without a bbg state");
}

// ── hash blocks (tag 15) ─────────────────────────────────────────────────────

/// Compile the hash.tri fixture (trident's hash builtin -> pattern 15).
fn compiled_hash() -> ProgramBundle {
    let mut options = trident::CompileOptions::for_profile("debug");
    options.target_config = joy_rs::nox_terrain();
    trident::compile_to_bundle(&fixture("hash.tri"), &options).expect("compile failed")
}

#[test]
fn hash_program_proves_and_verifies() {
    let warrior = Warrior::new();
    let b = compiled_hash();
    let (artifact, result) = warrior
        .prove_zheng(&b, &input(&[42], &[]))
        .expect("hash prove failed");
    assert_eq!(result.output.len(), 4, "hash returns a 4-limb digest");
    assert!(
        warrior.verify_zheng(&b, &artifact).expect("verify errored"),
        "hash proof must verify"
    );
}

#[test]
fn tampered_hash_group_rejected() {
    let warrior = Warrior::new();
    let b = compiled_hash();
    let (artifact, _) = warrior
        .prove_zheng(&b, &input(&[42], &[]))
        .expect("hash prove failed");

    // Tamper the witness commitment of an accumulator group via the wire
    // form (the rate itself is not in the artifact — it is bound through
    // the folded hash-binding steps, so any group tamper breaks the
    // cross-group linkage digest).
    let mut v: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&artifact).unwrap()).unwrap();
    let groups = v["proof"]["groups"].as_array().unwrap().len();
    let wc = &mut v["proof"]["groups"][groups - 1][1]["witness_commitment"];
    let b0 = wc[0].as_u64().unwrap();
    wc[0] = serde_json::Value::from(b0 ^ 1);
    let tampered: joy_rs::ProofArtifact = serde_json::from_value(v).unwrap();
    assert!(
        !warrior.verify_zheng(&b, &tampered).expect("verify errored"),
        "tampered hash proof must be rejected"
    );
}

// ── look rows (tag 17) against a real BBG state ──────────────────────────────
// trident cannot express a state read yet (os.state.read is unlowered), so
// the program is a hand-built .nox: a compose that first CONSES the look
// object carrying the state root limbs, then runs the look formula against
// it. Noted for the trident follow-up.

/// `[2 [[cons-tree of root limbs] [1 [17 [[1 ns] [1 key]]]]]]`
fn look_assembly(root: &[u8; 32], ns: u64, key: u64) -> String {
    let l = bbg::dim::goldilocks_from_bytes32(root);
    let (l0, l1, l2, l3) = (l[0].as_u64(), l[1].as_u64(), l[2].as_u64(), l[3].as_u64());
    format!(
        "[2 [[3 [[3 [[1 {l0}] [3 [[1 {l1}] [3 [[1 {l2}] [1 {l3}]]]]]]] [1 0]]] \
         [1 [17 [[1 {ns}] [1 {key}]]]]]]"
    )
}

/// A BBG state with two particles (mirrors bbg's look_e2e sample).
fn sample_state() -> bbg::BbgState {
    use bbg::types::ParticleRecord;
    let mut state = bbg::BbgState::new();
    state.particles.insert(
        [1u8; 32],
        ParticleRecord { energy: 77, pi_star: 0, weight: 0, s_yes: 0, s_no: 0, meta_score: 0 },
    );
    state.particles.insert(
        [2u8; 32],
        ParticleRecord { energy: 88, pi_star: 0, weight: 0, s_yes: 0, s_no: 0, meta_score: 0 },
    );
    state
}

#[test]
fn look_program_proves_and_verifies_against_state() {
    let state = sample_state();
    let root = state.root();
    // Particles dimension: cell 4 = first entry's energy (77).
    let b = bundle(&look_assembly(&root, 0, 4));
    let warrior = Warrior::new();
    let (artifact, result) = warrior
        .prove_zheng_with_state(&b, &input(&[], &[]), &state)
        .expect("look prove failed");
    assert_eq!(result.output, vec![77], "the look read the committed energy");
    assert_eq!(artifact.statement.bbg_root, root, "public root in the statement");
    assert!(
        warrior.verify_zheng(&b, &artifact).expect("verify errored"),
        "look proof must verify"
    );
}

#[test]
fn look_against_stale_root_refused_at_prove() {
    let state = sample_state();
    let stale_root = state.root();

    // State advances; the program still declares the stale root.
    let mut state = state;
    state.particles.insert(
        [3u8; 32],
        bbg::types::ParticleRecord { energy: 99, pi_star: 0, weight: 0, s_yes: 0, s_no: 0, meta_score: 0 },
    );
    state.refresh_root();

    let b = bundle(&look_assembly(&stale_root, 0, 4));
    let err = Warrior::new().prove_zheng_with_state(&b, &input(&[], &[]), &state);
    assert!(err.is_err(), "a stale declared root must not prove");
}

#[test]
fn look_artifact_root_mismatch_rejected() {
    let state = sample_state();
    let root = state.root();
    let b = bundle(&look_assembly(&root, 0, 4));
    let warrior = Warrior::new();
    let (artifact, _) = warrior
        .prove_zheng_with_state(&b, &input(&[], &[]), &state)
        .expect("look prove failed");

    // Flip a byte of the public root in the artifact.
    let mut v: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&artifact).unwrap()).unwrap();
    let b0 = v["statement"]["bbg_root"][0].as_u64().unwrap();
    v["statement"]["bbg_root"][0] = serde_json::Value::from((b0 ^ 1) & 0xff);
    let tampered: joy_rs::ProofArtifact = serde_json::from_value(v).unwrap();
    assert!(
        !warrior.verify_zheng(&b, &tampered).expect("verify errored"),
        "a proof re-rooted to a different state must be rejected"
    );
}

#[test]
fn look_artifact_tampered_binding_group_rejected() {
    let state = sample_state();
    let root = state.root();
    let b = bundle(&look_assembly(&root, 0, 4));
    let warrior = Warrior::new();
    let (artifact, _) = warrior
        .prove_zheng_with_state(&b, &input(&[], &[]), &state)
        .expect("look prove failed");

    // The leaves live in the folded binding steps; tamper the eq-group
    // witness commitment via the wire form — the cross-group linkage breaks.
    let mut v: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&artifact).unwrap()).unwrap();
    let groups = v["proof"]["groups"].as_array().unwrap().len();
    let idx = (0..groups)
        .find(|&i| v["proof"]["groups"][i][1]["committed_instance"]["num_cols"] == 3)
        .expect("an eq-step binding group exists");
    let wc = &mut v["proof"]["groups"][idx][1]["witness_commitment"];
    let b0 = wc[0].as_u64().unwrap();
    wc[0] = serde_json::Value::from((b0 ^ 1) & 0xff);
    let tampered: joy_rs::ProofArtifact = serde_json::from_value(v).unwrap();
    assert!(
        !warrior.verify_zheng(&b, &tampered).expect("verify errored"),
        "a tampered look binding group must be rejected"
    );
}
