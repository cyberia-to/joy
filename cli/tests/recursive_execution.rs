//! `joy prove --succinct` on a program the one-relation compiler refuses
//! (a loop nested deeper than its 128 levels) proves it recursively: a
//! profile-5 envelope (IVC over the nox machine, constant size) that
//! `joy verify` accepts self-contained and bound to its program, and
//! rejects with a wrong claim, input, program, budget or a flipped byte.
//! With too little memory for the wrap chain (`JOY_AVAILABLE_MEMORY=0`)
//! joy says so and proves profile 5; the verifying keys land in the cache
//! directory and a later `joy verify` installs them.
//! Proving takes ~1–2 min in release.
use joy_rs::RecursiveExecutionArtifact;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("joy-recursive-{}", std::process::id()));
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
            .env("JOY_AVAILABLE_MEMORY", "0")
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
fn a_program_beyond_one_relation_proves_recursively() {
    let f = Fixture::new();
    let run = f.ok(&["run", "loop.tri", "--input-values", "3"]);
    let expected = String::from_utf8_lossy(&run.stdout).lines().next().unwrap().trim().to_string();
    let proved = f.ok(&[
        "prove", "loop.tri", "--input-values", "3", "--succinct", "--output", "r.zheng",
    ]);
    let stderr = String::from_utf8_lossy(&proved.stderr);
    assert!(stderr.contains("profile 5"), "{stderr}");
    assert!(stderr.contains("--wrap forces it"), "the memory fallback is announced: {stderr}");
    let keys = std::fs::read_dir(f.0.join("cache/keys")).unwrap().count();
    assert_eq!(keys, 1, "the prover caches its verifying keys");
    let bytes = fs::read(f.0.join("r.zheng")).unwrap();
    assert_eq!(&bytes[..8], b"ZHENGPF1");
    assert_eq!(bytes[10], 5, "profile byte: recursive");
    let artifact = RecursiveExecutionArtifact::from_bytes(&bytes).unwrap();
    assert_eq!(artifact.output().unwrap(), vec![expected.parse::<u64>().unwrap()]);
    assert_eq!(artifact.to_bytes().unwrap(), bytes, "canonical");
    artifact.verify().unwrap();

    let out = f.ok(&["verify", "r.zheng", "--claim", &expected, "--input-values", "3"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("recursive public execution"), "{stdout}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("keys: "), "keys from the cache");
    // a corrupted cache file is refused, the keys derived and rewritten
    let file = std::fs::read_dir(f.0.join("cache/keys")).unwrap().next().unwrap().unwrap().path();
    std::fs::write(&file, b"not a key bundle").unwrap();
    let out = f.ok(&["verify", "r.zheng", "--claim", &expected]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("keys: derived, cached at"));
    assert!(std::fs::read(&file).unwrap().starts_with(b"ZHKEYS01"));
    f.ok(&["verify", "loop.tri", "--proof", "r.zheng", "--claim", &expected]);
    for args in [
        vec!["verify", "r.zheng", "--claim", "1"],
        vec!["verify", "r.zheng", "--input-values", "4"],
        vec!["verify", "r.zheng", "--budget", "1"],
        vec!["verify", "other.tri", "--proof", "r.zheng"],
    ] {
        assert!(!f.run(&args).status.success(), "{args:?}");
    }
    for i in [bytes.len() / 3, bytes.len() / 2, bytes.len() - 9] {
        let mut bad = bytes.clone();
        bad[i] ^= 1;
        fs::write(f.0.join("bad.zheng"), &bad).unwrap();
        assert!(!f.run(&["verify", "bad.zheng"]).status.success(), "byte {i}");
    }
    let mut longer = bytes.clone();
    longer.push(0);
    fs::write(f.0.join("long.zheng"), &longer).unwrap();
    assert!(!f.run(&["verify", "long.zheng"]).status.success());
}

#[test]
fn wrap_flags_need_succinct_and_exclude_each_other() {
    let f = Fixture::new();
    for args in [
        vec!["prove", "loop.tri", "--wrap"],
        vec!["prove", "loop.tri", "--no-wrap"],
        vec!["prove", "loop.tri", "--succinct", "--wrap", "--no-wrap"],
        vec!["prove", "loop.tri", "--succinct", "--machine", "--wrap"],
    ] {
        let out = f.run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}
