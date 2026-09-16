//! Function terminal branches must bind their actual selected result in proofs.
use joy_rs::{ExecutionArtifact, Warrior, ZkExecutionArtifact};
use trident::runtime::{ProgramBundle, ProgramInput};

fn compile(source: &str, profile: &str, name: &str) -> ProgramBundle {
    let directory = std::env::temp_dir().join(format!(
        "joy-terminal-{}-{name}-{profile}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("entry.tri");
    std::fs::write(&path, source).unwrap();
    let bundle =
        trident::compile_to_bundle(&path, &trident::CompileOptions::for_profile(profile)).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    bundle
}

#[test]
fn terminal_branch_public_certificates_bind_selected_values() {
    let source = "program terminal
fn choose(x: Field) -> Field {
    if x == 0 { let value = 7\n value } else { 9 }
}
fn main(x: Field) -> Field { choose(x) }";
    for profile in ["debug", "release"] {
        let bundle = compile(source, profile, "public");
        for (input, expected) in [(0, 7), (1, 9)] {
            let input = ProgramInput {
                public: vec![input],
                secret: vec![],
                digests: vec![],
            };
            let (proof, native) = Warrior::new()
                .prove_execution(&bundle, &input, 100_000)
                .unwrap();
            assert_eq!(native.output, vec![expected]);
            let decoded = ExecutionArtifact::from_bytes(&proof.to_bytes().unwrap()).unwrap();
            decoded.verify().unwrap();
            assert_eq!(decoded.statement.public_output, vec![expected]);
            for mutation in 0..3 {
                let mut bad = decoded.clone();
                match mutation {
                    0 => bad.statement.public_output[0] = 0,
                    1 => bad.statement.public_input[0] = 1 - input.public[0],
                    _ => bad.statement.cycles += 1,
                }
                assert!(bad.verify().is_err(), "mutation {mutation} accepted");
            }
        }
    }
}

#[test]
fn terminal_branch_private_stark_binds_witness_and_public_result() {
    let source = "program terminal_private
fn main(x: Field) -> Field {
    let witness: Field = divine()
    if x == 0 { witness + 7 } else { witness + 9 }
}";
    let bundle = compile(source, "release", "private");
    let input = ProgramInput {
        public: vec![0],
        secret: vec![35],
        digests: vec![],
    };
    let (proof, native) = Warrior::new()
        .prove_zk_execution(&bundle, &input, 100_000)
        .unwrap();
    assert_eq!(native.output, vec![42]);
    let decoded = ZkExecutionArtifact::from_bytes(&proof.to_bytes().unwrap()).unwrap();
    decoded.verify().unwrap();
    assert_eq!(decoded.statement.execution.public_output, vec![42]);
    for mutation in 0..3 {
        let mut bad = decoded.clone();
        match mutation {
            0 => bad.statement.execution.public_output[0] = 0,
            1 => bad.statement.execution.public_input[0] = 1,
            _ => bad.statement.execution.cycles += 1,
        }
        assert!(
            bad.verify().is_err(),
            "private mutation {mutation} accepted"
        );
    }
}
