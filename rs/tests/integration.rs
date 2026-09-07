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
fn prove_is_an_honest_dash() {
    let err = Warrior::new()
        .prove(&bundle("[1 0]"), &input(&[], &[]))
        .expect_err("prove must not pretend");
    assert!(err.contains("M4"), "unexpected error: {}", err);
}

#[test]
fn proof_verify_is_an_honest_dash() {
    let proof = trident::runtime::ProofData {
        claim: trident::field::proof::Claim {
            program_hash: Vec::new(),
            public_input: Vec::new(),
            public_output: Vec::new(),
        },
        proof_bytes: Vec::new(),
        format: "zheng-nox-v0".to_string(),
    };
    let err = Warrior::new()
        .verify(&proof)
        .expect_err("verify must not pretend");
    assert!(err.contains("M4"), "unexpected error: {}", err);
}
