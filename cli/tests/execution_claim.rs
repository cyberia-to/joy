use joy_rs::ExecutionArtifact;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("joy-execution-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("main.tri"),
            "program output_claim\nfn main(x: Field, y: Field) -> Field { (x + y) * x }\n",
        )
        .unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Output {
        let result = self.run(args);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result
    }
    fn prove(&self) {
        self.ok(&[
            "prove",
            "main.tri",
            "--input-values",
            "3,5",
            "--output",
            "proof.zheng",
        ]);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn fresh_verifier_authenticates_requested_public_io_without_execution_inputs() {
    let f = Fixture::new();
    f.prove();
    let output = f.ok(&[
        "verify",
        "proof.zheng",
        "--claim",
        "24",
        "--input-values",
        "3,5",
    ]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("public output: [24]"));
    f.ok(&[
        "verify",
        "main.tri",
        "--proof",
        "proof.zheng",
        "--claim",
        "24",
    ]);
    for extra in [
        ["--claim", "25"],
        ["--input-values", "4,5"],
        ["--budget", "1"],
    ] {
        assert!(!f
            .run(&["verify", "proof.zheng", extra[0], extra[1]])
            .status
            .success());
    }
}

#[test]
fn changing_statement_values_cannot_reauthorize_an_execution() {
    let f = Fixture::new();
    f.prove();
    let path = f.0.join("proof.zheng");
    let original = ExecutionArtifact::load(&path).unwrap();
    for change in 0..5 {
        let mut forged = original.clone();
        match change {
            0 => forged.statement.public_output = vec![999],
            1 => forged.statement.public_input = vec![4, 5],
            2 => forged.statement.cycles += 1,
            3 => forged.statement.budget += 1,
            _ => forged.assembly = "[1 999]".into(),
        }
        forged.save(&path).unwrap();
        assert!(
            !f.run(&["verify", "proof.zheng"]).status.success(),
            "forgery {change}"
        );
    }
}

#[test]
fn excess_secret_inputs_are_refused_and_single_quote_is_provable() {
    let f = Fixture::new();
    assert!(!f
        .run(&[
            "prove",
            "main.tri",
            "--input-values",
            "3,5",
            "--secret",
            "123456",
            "--output",
            "private.zheng"
        ])
        .status
        .success());
    assert!(!f.0.join("private.zheng").exists());
    fs::write(f.0.join("quote.nox"), "[1 42]").unwrap();
    f.ok(&["prove", "quote.nox", "--output", "quote.zheng"]);
    f.ok(&["verify", "quote.zheng", "--claim", "42"]);
}

#[test]
fn malformed_execution_artifacts_fail_directly_and_legacy_needs_opt_in() {
    let f = Fixture::new();
    f.prove();
    let path = f.0.join("proof.zheng");
    let mut bytes = fs::read(&path).unwrap();
    bytes.push(0);
    fs::write(&path, bytes).unwrap();
    let bad = f.run(&["verify", "proof.zheng"]);
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("Verification: FAIL"));
    let bundle =
        trident::compile_to_bundle(&f.0.join("main.tri"), &trident::CompileOptions::default())
            .unwrap();
    let input = trident::runtime::ProgramInput {
        public: vec![3, 5],
        secret: vec![],
        digests: vec![],
    };
    let (legacy, _) = joy_rs::Warrior::new().prove_zheng(&bundle, &input).unwrap();
    legacy.save(&f.0.join("legacy.zheng")).unwrap();
    assert!(!f.run(&["verify", "legacy.zheng"]).status.success());
    f.ok(&["verify", "legacy.zheng", "--legacy-trace-statement"]);
    assert!(!f
        .run(&[
            "verify",
            "legacy.zheng",
            "--legacy-trace-statement",
            "--input-values",
            "4,5"
        ])
        .status
        .success());
}

#[test]
fn private_proof_is_verified_in_fresh_process_without_secret() {
    let f = Fixture::new();
    fs::write(f.0.join("private.nox"), "[16 [[1 0] [1 0]]]").unwrap();
    f.ok(&[
        "prove",
        "private.nox",
        "--secret",
        "42",
        "--output",
        "private.zheng",
    ]);
    let artifact = joy_rs::ZkExecutionArtifact::load(&f.0.join("private.zheng")).unwrap();
    let output = f.ok(&["verify", "private.zheng", "--claim", "42"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Triton ZK"));
    f.ok(&[
        "verify",
        "private.nox",
        "--proof",
        "private.zheng",
        "--claim",
        "42",
    ]);
    assert!(!f
        .run(&["verify", "private.zheng", "--claim", "43"])
        .status
        .success());
    let mut forged = artifact;
    forged.statement.execution.public_output[0] = 43;
    forged.save(&f.0.join("private.zheng")).unwrap();
    assert!(!f
        .run(&["verify", "private.zheng", "--claim", "43"])
        .status
        .success());
    fs::write(f.0.join("quote.nox"), "[1 42]").unwrap();
    f.ok(&["prove", "quote.nox", "--zk", "--output", "quote-zk.zheng"]);
    f.ok(&["verify", "quote-zk.zheng", "--claim", "42"]);
}
