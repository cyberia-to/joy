//! A source-level loop return must survive native execution and the proved CCS.
use joy_rs::{ExecutionArtifact, Warrior};
use trident::runtime::ProgramInput;

#[test]
fn bounded_loop_return_proof_binds_selected_result_input_and_cost() {
    let directory = std::env::temp_dir().join(format!("joy-loop-proof-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("return.tri");
    std::fs::write(&source, "program returning\nfn main(n: Field) -> Field {\n let mut total: Field = 10\n for i in 0..n bounded 3 {\n if as_field(i) == 1 { return total }\n total = total + 2\n }\n let later = total + 100\n later\n}\n").unwrap();
    let warrior = Warrior::new();
    for profile in ["debug", "release"] {
        let options = trident::CompileOptions::for_profile(profile);
        let bundle = trident::compile_to_bundle(&source, &options).unwrap();
        for (n, expected) in [(0, 110), (1, 112), (3, 12)] {
            let input = ProgramInput {
                public: vec![n],
                secret: vec![],
                digests: vec![],
            };
            let (artifact, native) = warrior.prove_execution(&bundle, &input, 100_000).unwrap();
            assert_eq!(native.output, vec![expected]);
            let decoded = ExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
            decoded.verify().unwrap();
            assert_eq!(decoded.statement.public_output, vec![expected]);
            assert_eq!(decoded.statement.cycles, native.cycle_count);
            for mutation in 0..3 {
                let mut bad = decoded.clone();
                match mutation {
                    0 => bad.statement.public_output[0] += 1,
                    1 => bad.statement.public_input[0] += 1,
                    _ => bad.statement.cycles += 1,
                }
                assert!(bad.verify().is_err(), "mutation {mutation} accepted");
            }
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn private_loop_return_skips_later_witness_and_binds_real_stark() {
    let directory =
        std::env::temp_dir().join(format!("joy-private-loop-proof-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("private.tri");
    std::fs::write(&source, "program private_return\nfn main(n: Field) -> Field {\n let witness: Field = divine()\n for i in 0..2 { if as_field(i) == 1 { return witness + n } }\n let unused: Field = divine()\n unused\n}\n").unwrap();
    let bundle =
        trident::compile_to_bundle(&source, &trident::CompileOptions::for_profile("release"))
            .unwrap();
    let input = ProgramInput {
        public: vec![5],
        secret: vec![37],
        digests: vec![],
    };
    let (artifact, native) = Warrior::new()
        .prove_zk_execution(&bundle, &input, 100_000)
        .unwrap();
    assert_eq!(native.output, vec![42]);
    let decoded = joy_rs::ZkExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
    decoded.verify().unwrap();
    for mutation in 0..3 {
        let mut bad = decoded.clone();
        match mutation {
            0 => bad.statement.execution.public_output[0] += 1,
            1 => bad.statement.execution.public_input[0] += 1,
            _ => bad.statement.execution.cycles += 1,
        }
        assert!(
            bad.verify().is_err(),
            "private mutation {mutation} accepted"
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}
