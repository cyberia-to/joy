#![cfg(unix)]
use joy_rs::{
    state_execution::load_certificate, ExecutionArtifact, ProofArtifact, StateExecutionArtifact,
    ZkExecutionArtifact,
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("joy-artifact-files-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("main.tri"),
            "program file_check\nfn main()->Field {7}\n",
        )
        .unwrap();
        Self(root)
    }
    fn run(&self, args: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_joy"))
            .args(args)
            .current_dir(&self.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let start = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if start.elapsed() > Duration::from_secs(3) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("verification blocked on file input: {args:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.wait_with_output().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn verification_rejects_streams_links_and_oversize_files_but_keeps_real_proof_roundtrip() {
    let f = Fixture::new();
    let fifo = f.0.join("pipe.nox");
    assert!(Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    std::os::unix::fs::symlink(&fifo, f.0.join("pipe-link.zheng")).unwrap();
    for name in ["pipe.nox", "pipe-link.zheng"] {
        for args in [
            vec!["verify", name],
            vec!["verify", "main.tri", "--proof", name],
        ] {
            let result = f.run(&args);
            assert_eq!(result.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&result.stderr).contains("regular file"));
        }
    }
    let oversized = f.0.join("oversized.zheng");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    for name in ["oversized.json", "oversized.nox"] {
        fs::hard_link(&oversized, f.0.join(name)).unwrap();
        let result = f.run(&["verify", name, "--claim", "7"]);
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("size limit"));
    }
    for path in [&fifo, &f.0, &f.0.join("pipe-link.zheng"), &oversized] {
        assert!(!ExecutionArtifact::has_header(path));
        assert!(!ZkExecutionArtifact::has_header(path));
        assert!(!StateExecutionArtifact::has_header(path));
        assert!(ExecutionArtifact::load(path).is_err());
        assert!(ZkExecutionArtifact::load(path).is_err());
        assert!(StateExecutionArtifact::load(path).is_err());
        assert!(ProofArtifact::load(path).is_err());
        assert!(load_certificate(path).is_err());
    }
    let proved = f.run(&["prove", "main.tri", "--output", "proof.zheng"]);
    assert!(
        proved.status.success(),
        "{}",
        String::from_utf8_lossy(&proved.stderr)
    );
    assert!(ExecutionArtifact::has_header(&f.0.join("proof.zheng")));
    for args in [
        vec!["verify", "proof.zheng", "--claim", "7"],
        vec![
            "verify",
            "main.tri",
            "--proof",
            "proof.zheng",
            "--claim",
            "7",
        ],
    ] {
        let verified = f.run(&args);
        assert!(
            verified.status.success(),
            "{}",
            String::from_utf8_lossy(&verified.stderr)
        );
    }
    assert_eq!(
        f.run(&["verify", "proof.zheng", "--claim", "8"])
            .status
            .code(),
        Some(1)
    );
    std::os::unix::fs::symlink(f.0.join("proof.zheng"), f.0.join("proof-link.zheng")).unwrap();
    assert_eq!(
        f.run(&["verify", "proof-link.zheng"]).status.code(),
        Some(1)
    );
}
