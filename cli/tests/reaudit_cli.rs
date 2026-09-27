use std::{
    ffi::OsString,
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
            "joy-cli-reaudit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("ok.nox"), "[1 7]").unwrap();
        Self(path)
    }
    fn run(&self, args: &[OsString]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[OsString]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
#[cfg(target_os = "linux")]
fn option(name: &str, value: &std::ffi::OsStr) -> OsString {
    let mut result = OsString::from(name);
    result.push("=");
    result.push(value);
    result
}

#[test]
fn batch_preserves_option_looking_inputs_witnesses_and_output_paths() {
    let f = Fixture::new();
    fs::write(f.0.join("--first.nox"), "[1 11]").unwrap();
    fs::write(f.0.join("--second.nox"), "[1 13]").unwrap();
    fs::write(
        f.0.join("--witness.json"),
        r#"{"schema_version":1,"public":[],"secret":[]}"#,
    )
    .unwrap();
    let result = f.ok(&args(&[
        "batch",
        "run",
        "--input-file=--witness.json",
        "--max-parallel=2",
        "--",
        "--first.nox",
        "--second.nox",
    ]));
    assert_eq!(result.stdout, b"11\n13\n");
    f.ok(&args(&[
        "batch",
        "prove",
        "ok.nox",
        "--input-file=--witness.json",
        "--output=--proofs",
    ]));
    let artifact = f.0.join("--proofs/0000/proof.zheng");
    assert!(fs::read(&artifact).unwrap().starts_with(b"JOYEXEC2"));
    f.ok(&[OsString::from("verify"), artifact.into_os_string()]);
}

// Linux filesystems admit these names; macOS rejects them before CLI invocation.
#[cfg(target_os = "linux")]
#[test]
fn batch_keeps_non_utf8_option_values_in_native_arguments() {
    use std::os::unix::ffi::OsStringExt;
    let f = Fixture::new();
    let witness = OsString::from_vec(b"--witness-\xff.json".to_vec());
    let directory = OsString::from_vec(b"--proofs-\xfe".to_vec());
    fs::write(
        f.0.join(&witness),
        r#"{"schema_version":1,"public":[],"secret":[]}"#,
    )
    .unwrap();
    let mut invocation = args(&["batch", "prove", "ok.nox"]);
    invocation.push(option("--input-file", &witness));
    invocation.push(option("--output", &directory));
    f.ok(&invocation);
    let artifact = f.0.join(directory).join("0000/proof.zheng");
    f.ok(&[OsString::from("verify"), artifact.into_os_string()]);
}

#[test]
fn witness_file_byte_limit_accepts_exact_size_and_rejects_growth() {
    let f = Fixture::new();
    let mut bytes = br#"{"schema_version":1,"public":[],"secret":[]}"#.to_vec();
    bytes.resize(4 * 1024 * 1024, b' ');
    let path = f.0.join("input.json");
    fs::write(&path, &bytes).unwrap();
    f.ok(&args(&["run", "ok.nox", "--input-file", "input.json"]));
    bytes.push(b' ');
    fs::write(path, bytes).unwrap();
    let result = f.run(&args(&["run", "ok.nox", "--input-file", "input.json"]));
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("4 MiB"));
}

#[cfg(unix)]
#[test]
fn forced_proof_publication_replaces_the_link_and_preserves_its_target() {
    let f = Fixture::new();
    let target = f.0.join("keep.txt");
    let link = f.0.join("proof.zheng");
    fs::write(&target, b"keep original").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let rejected = f.run(&args(&["prove", "ok.nox", "--output", "proof.zheng"]));
    assert!(!rejected.status.success());
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    f.ok(&args(&[
        "prove",
        "ok.nox",
        "--output",
        "proof.zheng",
        "--force",
    ]));
    assert!(fs::symlink_metadata(&link).unwrap().is_file());
    assert_eq!(fs::read(target).unwrap(), b"keep original");
    f.ok(&args(&["verify", "proof.zheng"]));
}

#[cfg(unix)]
#[test]
fn source_commands_reject_entry_import_and_manifest_streams_before_publication() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    fn fifo(path: &std::path::Path) {
        assert!(Command::new("mkfifo").arg(path).status().unwrap().success());
    }
    let rejected = |invocation: &[&str]| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_joy"))
            .current_dir(&f.0)
            .args(invocation)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("source command blocked before execution admission: {invocation:?}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("regular file"), "{error}");
    };
    fifo(&f.0.join("pipe.tri"));
    fs::write(f.0.join("keep.out"), b"preserve previous result").unwrap();
    for invocation in [
        vec!["run", "pipe.tri", "--budget", "1"],
        vec!["build", "pipe.tri", "--output", "keep.out", "--force"],
        vec!["prove", "pipe.tri", "--output", "keep.out", "--force"],
        vec!["bench", "pipe.tri", "--repeat", "1"],
        vec!["test", "pipe.tri"],
        vec!["verify", "pipe.tri", "--claim", "7"],
    ] {
        rejected(&invocation);
    }
    assert_eq!(
        fs::read(f.0.join("keep.out")).unwrap(),
        b"preserve previous result"
    );
    fs::create_dir(f.0.join("project")).unwrap();
    fifo(&f.0.join("project/trident.toml"));
    rejected(&["run", "project", "--budget", "1"]);
    fs::create_dir(f.0.join("imports")).unwrap();
    fs::write(
        f.0.join("imports/main.tri"),
        "program app use helper fn main()->Field{helper.value()}",
    )
    .unwrap();
    fifo(&f.0.join("imports/helper.tri"));
    rejected(&["run", "imports/main.tri", "--budget", "1"]);
}
