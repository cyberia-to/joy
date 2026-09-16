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
        let path = std::env::temp_dir().join(format!("joy-state-exec-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("main.tri"),"program state_test\nfn helper(k: Field) -> Field { os.state.read(k) }\nfn main(k: Field) -> Field { helper(k) + 5 }\n").unwrap();
        let mut state = bbg::BbgState::new();
        let mut record = bbg::types::ParticleRecord::zero();
        record.energy = 77;
        state.particles.insert([1; 32], record);
        let certificate = bbg::certificate::StateCertificate::from_state(&state, &[0]).unwrap();
        fs::write(
            path.join("partial.json"),
            serde_json::to_vec(&certificate).unwrap(),
        )
        .unwrap();
        let certificate =
            bbg::certificate::StateCertificate::from_state(&state, &(0..10).collect::<Vec<_>>())
                .unwrap();
        fs::write(
            path.join("state.json"),
            serde_json::to_vec(&certificate).unwrap(),
        )
        .unwrap();
        state.particles.get_mut(&[1; 32]).unwrap().energy = 88;
        state.refresh_root();
        let other = bbg::certificate::StateCertificate::from_state(&state, &[0]).unwrap();
        fs::write(path.join("other.json"), serde_json::to_vec(&other).unwrap()).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Output {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn compiled_helper_state_proof_verifies_in_a_fresh_process_and_rejects_substitution() {
    let f = Fixture::new();
    let run = f.ok(&[
        "run",
        "main.tri",
        "--state",
        "state.json",
        "--input-values",
        "11",
    ]);
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "82");
    f.ok(&[
        "prove",
        "main.tri",
        "--state",
        "state.json",
        "--input-values",
        "11",
        "--output",
        "proof.zheng",
    ]);
    for args in [
        vec![
            "verify",
            "proof.zheng",
            "--claim",
            "82",
            "--input-values",
            "11",
        ],
        vec![
            "verify",
            "main.tri",
            "--proof",
            "proof.zheng",
            "--state",
            "state.json",
            "--claim",
            "82",
        ],
    ] {
        f.ok(&args);
    }
    for args in [
        vec!["verify", "proof.zheng", "--claim", "83"],
        vec!["verify", "proof.zheng", "--input-values", "12"],
        vec!["verify", "proof.zheng", "--state", "other.json"],
        vec![
            "prove",
            "main.tri",
            "--state",
            "partial.json",
            "--zk",
            "--input-values",
            "11",
            "--output",
            "bad.zheng",
        ],
    ] {
        assert!(!f.run(&args).status.success(), "accepted {args:?}");
    }
    assert!(!f.0.join("bad.zheng").exists());
    let bytes = fs::read(f.0.join("proof.zheng")).unwrap();
    let mut forged = joy_rs::StateExecutionArtifact::from_bytes(&bytes).unwrap();
    forged.certificate.dimensions[0].fields[11] += 1;
    forged.save(&f.0.join("forged.zheng")).unwrap();
    assert!(!f.run(&["verify", "forged.zheng"]).status.success());
}

#[test]
fn private_state_query_is_proved_and_fresh_verifier_needs_no_key() {
    let f = Fixture::new();
    fs::write(f.0.join("private.tri"),"program hidden_lookup\nfn helper(k: Field) -> Field { os.state.read(k) }\nfn main() -> Field { let k: Field = divine()\n helper(k) + 5 }\n").unwrap();
    f.ok(&[
        "prove",
        "private.tri",
        "--state",
        "state.json",
        "--secret",
        "11",
        "--output",
        "private.zheng",
    ]);
    let bytes = fs::read(f.0.join("private.zheng")).unwrap();
    let artifact = joy_rs::ZkExecutionArtifact::from_bytes(&bytes).unwrap();
    assert_eq!(artifact.statement.execution.public_input, Vec::<u64>::new());
    assert_eq!(artifact.statement.execution.public_output, vec![82]);
    assert!(artifact.state.is_some());
    let json = serde_json::to_value(&artifact).unwrap();
    assert!(json["statement"]["reads"].is_null());
    f.ok(&["verify", "private.zheng", "--claim", "82"]);
    f.ok(&[
        "verify",
        "private.tri",
        "--proof",
        "private.zheng",
        "--state",
        "state.json",
        "--claim",
        "82",
    ]);
    assert!(!f
        .run(&["verify", "private.zheng", "--claim", "83"])
        .status
        .success());
    assert!(!f
        .run(&["verify", "private.zheng", "--state", "other.json"])
        .status
        .success());
    let mut forged = artifact.clone();
    forged.state.as_mut().unwrap().dimensions[0].fields[11] += 1;
    forged.save(&f.0.join("forged.zheng")).unwrap();
    assert!(!f.run(&["verify", "forged.zheng"]).status.success());
}
