use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "joy-native-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let f = Self(path);
        f.write("a.nox", "[1 11]");
        f.write("b.nox", "[1 22]");
        f.write(
            "secret.tri",
            "program witness\nfn main(x: Field) -> Field { let y: Field = divine()\n x * y }\n",
        );
        f
    }
    fn write(&self, path: &str, text: &str) {
        fs::write(self.0.join(path), text).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
    fn bad(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(!output.status.success());
        output
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn input_file_serves_native_divine_and_conflicts_with_explicit_flags() {
    let f = Fixture::new();
    f.write(
        "input.json",
        r#"{"schema_version":1,"public":["6"],"secret":[7]}"#,
    );
    let output = f.ok(&["run", "secret.tri", "--input-file", "input.json"]);
    assert_eq!(output.stdout, b"42\n");
    for flag in ["--input-values", "--secret"] {
        f.bad(&["run", "secret.tri", "--input-file", "input.json", flag, "8"]);
    }
    let output = f.ok(&[
        "verify",
        "secret.tri",
        "--claim",
        "42",
        "--input-file",
        "input.json",
    ]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS"));
}

#[test]
fn malformed_witnesses_never_echo_their_contents() {
    let f = Fixture::new();
    for input in [
        r#"{"schema_version":1,"public":[],"secret":["PRIVATE_SENTINEL"]}"#,
        r#"{"schema_version":1,"public":[],"secret":["18446744069414584321"]}"#,
        r#"{"schema_version":1,"public":[],"secret":["01"]}"#,
        r#"{"schema_version":1,"public":[],"secret":[-1]}"#,
        r#"{"schema_version":1,"public":[],"secret":[1.5]}"#,
        r#"{"schema_version":1,"public":[],"secret":[],"secret":[7]}"#,
        r#"{"schema_version":1,"public":[],"secret":[],"digests":[]}"#,
        r#"{"schema_version":2,"public":[],"secret":[]}"#,
        r#"{"schema_version":1,"public":[]}"#,
    ] {
        f.write("bad.json", input);
        let output = f.bad(&["run", "secret.tri", "--input-file", "bad.json"]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!text.contains("PRIVATE_SENTINEL"));
        assert!(!text.contains("18446744069414584321"));
    }
    let path = f.0.join("oversize.json");
    fs::File::create(path)
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    f.bad(&["run", "secret.tri", "--input-file", "oversize.json"]);
    f.bad(&["run", "secret.tri", "--input-file", "."]);
}

#[cfg(unix)]
#[test]
fn witness_links_and_streams_are_refused_without_opening() {
    let f = Fixture::new();
    f.write(
        "input.json",
        r#"{"schema_version":1,"public":[6],"secret":[7]}"#,
    );
    std::os::unix::fs::symlink(f.0.join("input.json"), f.0.join("link.json")).unwrap();
    f.bad(&["run", "secret.tri", "--input-file", "link.json"]);
    assert!(Command::new("mkfifo")
        .arg(f.0.join("pipe.json"))
        .status()
        .unwrap()
        .success());
    f.bad(&["run", "secret.tri", "--input-file", "pipe.json"]);
}

#[test]
fn batch_execution_preserves_order_and_failure_of_any_job() {
    let f = Fixture::new();
    let output = f.ok(&["batch", "run", "b.nox", "a.nox", "--max-parallel", "2"]);
    assert_eq!(output.stdout, b"22\n11\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.find("[0] PASS").unwrap() < stderr.find("[1] PASS").unwrap());
    f.write("broken.nox", "[bad syntax");
    let output = f.bad(&["batch", "run", "broken.nox", "a.nox", "--max-parallel", "2"]);
    assert_eq!(output.stdout, b"11\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("[0] FAIL"));
    for parallel in ["0", "9"] {
        f.bad(&["batch", "run", "a.nox", "--max-parallel", parallel]);
    }
    f.write(
        "input.json",
        r#"{"schema_version":1,"public":[6],"secret":[7]}"#,
    );
    let output = f.ok(&[
        "batch",
        "run",
        "secret.tri",
        "secret.tri",
        "--input-file",
        "input.json",
    ]);
    assert_eq!(output.stdout, b"42\n42\n");
}

#[test]
fn batch_proofs_use_isolated_paths_and_verify_each_artifact() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("one")).unwrap();
    fs::create_dir(f.0.join("two")).unwrap();
    f.write("one/main.nox", "[1 11]");
    f.write("two/main.nox", "[1 22]");
    f.ok(&[
        "batch",
        "prove",
        "one/main.nox",
        "two/main.nox",
        "--output",
        "proofs",
    ]);
    let first = f.0.join("proofs/0000/proof.zheng");
    let second = f.0.join("proofs/0001/proof.zheng");
    assert!(first.is_file() && second.is_file());
    let before = fs::read(&first).unwrap();
    f.write(
        "public.json",
        r#"{"schema_version":1,"public":[],"secret":[]}"#,
    );
    f.ok(&[
        "verify",
        "proofs/0000/proof.zheng",
        "--input-file",
        "public.json",
        "--claim",
        "11",
    ]);
    f.bad(&["batch", "prove", "a.nox", "--output", "proofs"]);
    assert_eq!(fs::read(&first).unwrap(), before);
    f.ok(&[
        "batch",
        "verify",
        "proofs/0000/proof.zheng",
        "proofs/0001/proof.zheng",
    ]);
    fs::write(&second, b"invalid proof").unwrap();
    f.bad(&[
        "batch",
        "verify",
        "proofs/0000/proof.zheng",
        "proofs/0001/proof.zheng",
    ]);
    f.ok(&["batch", "prove", "b.nox", "--output", "proofs", "--force"]);
    f.ok(&["verify", "proofs/0000/proof.zheng", "--claim", "22"]);
}

#[test]
fn batch_witness_preparation_failure_creates_no_proofs() {
    let f = Fixture::new();
    f.write(
        "input.json",
        r#"{"schema_version":1,"public":[],"secret":["PRIVATE_SENTINEL"]}"#,
    );
    let output = f.bad(&[
        "batch",
        "prove",
        "a.nox",
        "--input-file",
        "input.json",
        "--output",
        "proofs",
    ]);
    assert!(!f.0.join("proofs").exists());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_SENTINEL"));
}
