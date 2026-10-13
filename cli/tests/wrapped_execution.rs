//! `joy prove --succinct --wrap` on a program beyond one relation: a
//! profile-6 envelope (IVC, its final verifier wrapped, ≤ 64 KB) that
//! `joy verify` accepts self-contained and bound to its program, and
//! rejects with a wrong claim, input, program, budget or a flipped byte;
//! a fresh verify installs the keys the prover cached.
//! Ignored by default: the wrap chain takes ~1–3 min and ~15 GB.
use joy_rs::WrappedExecutionArtifact;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("joy-wrapped-{}", std::process::id()));
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
            .env("JOY_CACHE_DIR", self.0.join("cache"))
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
#[ignore = "the wrap chain: ~1–3 min, ~15 GB"]
fn a_program_beyond_one_relation_proves_wrapped() {
    let f = Fixture::new();
    let run = f.ok(&["run", "loop.tri", "--input-values", "3"]);
    let expected = String::from_utf8_lossy(&run.stdout).lines().next().unwrap().trim().to_string();
    let proved = f.ok(&[
        "prove", "loop.tri", "--input-values", "3", "--succinct", "--wrap", "--output", "w.zheng",
    ]);
    assert!(String::from_utf8_lossy(&proved.stderr).contains("profile 6"));
    let bytes = fs::read(f.0.join("w.zheng")).unwrap();
    assert_eq!(&bytes[..8], b"ZHENGPF1");
    assert_eq!(bytes[10], 6, "profile byte: wrapped");
    // the final proof is under zheng's hard gate over every transcript; the
    // envelope (proof + this fixture's statement) under 64 KiB
    let artifact = WrappedExecutionArtifact::from_bytes(&bytes).unwrap();
    assert!(artifact.proof_len().unwrap() <= joy_rs::wrapped_execution::MAX_FINAL_PROOF_BYTES);
    assert!(bytes.len() <= 64 * 1024, "{} B", bytes.len());

    let out = f.ok(&["verify", "loop.tri", "--proof", "w.zheng", "--claim", &expected, "--input-values", "3"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("wrapped public execution"), "{stdout}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("keys: "), "keys from the cache");
    f.ok(&["verify", "loop.tri", "--proof", "w.zheng", "--claim", &expected]);
    // the proof binds its program by digest: no program, no verification
    let alone = f.run(&["verify", "w.zheng"]);
    assert!(!alone.status.success());
    assert!(String::from_utf8_lossy(&alone.stderr).contains("binds its program by digest"));
    for args in [
        vec!["verify", "loop.tri", "--proof", "w.zheng", "--claim", "1"],
        vec!["verify", "loop.tri", "--proof", "w.zheng", "--input-values", "4"],
        vec!["verify", "loop.tri", "--proof", "w.zheng", "--budget", "1"],
        vec!["verify", "other.tri", "--proof", "w.zheng"],
    ] {
        assert!(!f.run(&args).status.success(), "{args:?}");
    }
    for i in [bytes.len() / 3, bytes.len() / 2, bytes.len() - 9] {
        let mut bad = bytes.clone();
        bad[i] ^= 1;
        fs::write(f.0.join("bad.zheng"), &bad).unwrap();
        assert!(!f.run(&["verify", "loop.tri", "--proof", "bad.zheng"]).status.success(), "byte {i}");
    }
    let mut longer = bytes.clone();
    longer.push(0);
    fs::write(f.0.join("long.zheng"), &longer).unwrap();
    assert!(!f.run(&["verify", "loop.tri", "--proof", "long.zheng"]).status.success());
    // in-process: canonical
    let artifact = WrappedExecutionArtifact::from_bytes(&bytes).unwrap();
    assert_eq!(artifact.output().unwrap(), vec![expected.parse::<u64>().unwrap()]);
    assert_eq!(artifact.to_bytes().unwrap(), bytes, "canonical");
}
