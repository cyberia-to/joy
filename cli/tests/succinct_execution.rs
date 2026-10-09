//! `joy prove --succinct` writes a zheng profile-1 envelope; `joy verify`
//! accepts it self-contained and bound to a program, and rejects a wrong
//! claim, input, program, budget, a flipped byte, and the flag combinations
//! the profile does not cover.
use joy_rs::SuccinctExecutionArtifact;
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
        let path = std::env::temp_dir().join(format!("joy-succinct-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("main.tri"),
            "program succinct_claim\nfn main(x: Field, y: Field) -> Field { (x + y) * x }\n",
        )
        .unwrap();
        fs::write(
            path.join("other.tri"),
            "program other\nfn main(x: Field, y: Field) -> Field { (x + y) * y }\n",
        )
        .unwrap();
        fs::write(
            path.join("hash.tri"),
            "program hashone\nfn main(a: Field) -> Digest {\n    hash(a, 0, 0, 0, 0, 0, 0, 0)\n}\n",
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn succinct_proofs_verify_and_bind_their_statement() {
    let f = Fixture::new();
    f.ok(&[
        "prove", "main.tri", "--input-values", "3,5", "--succinct", "--output", "p.zheng",
    ]);
    let bytes = fs::read(f.0.join("p.zheng")).unwrap();
    assert_eq!(&bytes[..8], b"ZHENGPF1");
    assert_eq!(bytes[10], 1, "profile byte: succinct");
    let artifact = SuccinctExecutionArtifact::from_bytes(&bytes).unwrap();
    assert_eq!(artifact.statement.public_output, vec![24]);
    artifact.verify().unwrap();

    let out = f.ok(&["verify", "p.zheng", "--claim", "24", "--input-values", "3,5"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("succinct public execution"), "{stdout}");
    assert!(stdout.contains("public output: [24]"));
    f.ok(&["verify", "main.tri", "--proof", "p.zheng", "--claim", "24"]);

    for args in [
        vec!["verify", "p.zheng", "--claim", "25"],
        vec!["verify", "p.zheng", "--input-values", "4,5"],
        vec!["verify", "p.zheng", "--budget", "1"],
        vec!["verify", "other.tri", "--proof", "p.zheng"],
    ] {
        assert!(!f.run(&args).status.success(), "{args:?}");
    }

    // a flipped byte anywhere in the opening fails verification
    let mut bad = bytes.clone();
    let i = bytes.len() - 100;
    bad[i] ^= 1;
    fs::write(f.0.join("bad.zheng"), &bad).unwrap();
    assert!(!f.run(&["verify", "bad.zheng"]).status.success());
}

#[test]
fn hash_tri_proves_and_verifies_succinctly() {
    let f = Fixture::new();
    f.ok(&["prove", "hash.tri", "--input-values", "7", "--succinct", "--output", "h.zheng"]);
    f.ok(&["verify", "h.zheng"]);
    f.ok(&["prove", "hash.tri", "--input-values", "7", "--output", "public.zheng"]);
    let succinct = fs::metadata(f.0.join("h.zheng")).unwrap().len();
    let public = fs::metadata(f.0.join("public.zheng")).unwrap().len();
    // the same statement both ways; recorded in zheng audit/succinct-profile-2026-10.md
    assert!(succinct > 0 && public > 0);
    println!("hash.tri: succinct {succinct} B, public v3 {public} B");
}

#[test]
fn succinct_refuses_secrets_and_zk() {
    let f = Fixture::new();
    for args in [
        vec!["prove", "main.tri", "--input-values", "3,5", "--succinct", "--zk"],
        vec!["prove", "main.tri", "--input-values", "3", "--secret", "5", "--succinct"],
    ] {
        assert!(!f.run(&args).status.success(), "{args:?}");
    }
}
