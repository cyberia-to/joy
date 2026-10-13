//! `joy prove --succinct --machine` on a program the one-relation compiler
//! refuses (a loop nested deeper than its 128 levels) falls back to zheng's
//! accumulated machine: a profile-4 envelope that `joy verify` accepts self-contained
//! and bound to its program, and rejects with a wrong claim, input,
//! program, budget or a flipped byte.
use joy_rs::MachineExecutionArtifact;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("joy-machine-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("loop.tri"),
            "program squares\nfn main(a: Field) -> Field {\n    let mut x: Field = a\n    for i in 0..140 bounded 140 {\n        x = x * x + 1\n    }\n    x\n}\n",
        )
        .unwrap();
        fs::write(
            path.join("other.tri"),
            "program cubes\nfn main(a: Field) -> Field {\n    let mut x: Field = a\n    for i in 0..140 bounded 140 {\n        x = x * x * x\n    }\n    x\n}\n",
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
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        result
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn machine_requires_succinct() {
    let f = Fixture::new();
    assert!(!f.run(&["prove", "loop.tri", "--input-values", "3", "--machine"]).status.success());
}

#[test]
fn a_program_beyond_one_relation_proves_with_the_machine() {
    let f = Fixture::new();
    let run = f.ok(&["run", "loop.tri", "--input-values", "3"]);
    let expected = String::from_utf8_lossy(&run.stdout).lines().next().unwrap().trim().to_string();
    f.ok(&[
        "prove", "loop.tri", "--input-values", "3", "--succinct", "--machine", "--output", "m.zheng",
    ]);
    let bytes = fs::read(f.0.join("m.zheng")).unwrap();
    assert_eq!(&bytes[..8], b"ZHENGPF1");
    assert_eq!(bytes[10], 4, "profile byte: machine");
    let artifact = MachineExecutionArtifact::from_bytes(&bytes).unwrap();
    assert_eq!(artifact.output().unwrap(), vec![expected.parse::<u64>().unwrap()]);

    let out = f.ok(&["verify", "loop.tri", "--proof", "m.zheng", "--claim", &expected, "--input-values", "3"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("machine public execution"), "{stdout}");
    f.ok(&["verify", "loop.tri", "--proof", "m.zheng", "--claim", &expected]);
    // the proof binds its program by digest: no program, no verification
    let alone = f.run(&["verify", "m.zheng"]);
    assert!(!alone.status.success());
    assert!(String::from_utf8_lossy(&alone.stderr).contains("binds its program by digest"));
    for args in [
        vec!["verify", "loop.tri", "--proof", "m.zheng", "--claim", "1"],
        vec!["verify", "loop.tri", "--proof", "m.zheng", "--input-values", "4"],
        vec!["verify", "loop.tri", "--proof", "m.zheng", "--budget", "1"],
        vec!["verify", "other.tri", "--proof", "m.zheng"],
    ] {
        assert!(!f.run(&args).status.success(), "{args:?}");
    }
    let mut bad = bytes.clone();
    let i = bytes.len() / 2;
    bad[i] ^= 1;
    fs::write(f.0.join("bad.zheng"), &bad).unwrap();
    assert!(!f.run(&["verify", "loop.tri", "--proof", "bad.zheng"]).status.success());
}
