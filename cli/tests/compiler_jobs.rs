use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture {
    path: PathBuf,
    vectors: Value,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "joy-compiler-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let vectors: Value = serde_json::from_str(include_str!("compiler_vectors.json")).unwrap();
        let f = Self { path, vectors };
        for (name, hex) in f.vectors["files"].as_object().unwrap() {
            let bytes: Vec<_> = hex
                .as_str()
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
                .collect();
            fs::write(f.path.join(name), bytes).unwrap();
        }
        f
    }
    fn run(&self, program: &str, input: &str, extra: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .arg("run-artifact")
            .arg(self.path.join(program))
            .arg("--input")
            .arg(self.path.join(input))
            .arg("-o")
            .arg(self.path.join("out"))
            .args(extra)
            .output()
            .unwrap()
    }
    fn no_staging(&self) {
        assert!(fs::read_dir(&self.path).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with('.')));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn compiler_result_and_extracted_program_are_published_exactly_and_executed() {
    let f = Fixture::new();
    let output = f.run("compiler", "job", &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["execution"]["compiler_job"]["status"], "success");
    assert_eq!(
        fs::read(f.path.join("out")).unwrap(),
        fs::read(f.path.join("result")).unwrap()
    );
    assert_eq!(
        report["published_particle"],
        report["execution"]["output_particle"]
    );
    let output = f.run("compiler", "job", &["--emit", "program", "--force"]);
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["published_particle"],
        report["execution"]["compiler_job"]["compiled_particle"]
    );
    assert_ne!(
        report["published_particle"],
        report["execution"]["output_particle"]
    );
    assert_eq!(
        fs::read(f.path.join("out")).unwrap(),
        fs::read(f.path.join("generated")).unwrap()
    );
    fs::rename(f.path.join("out"), f.path.join("compiled")).unwrap();
    assert!(f.run("compiled", "zero", &[]).status.success());
    assert_eq!(
        fs::read(f.path.join("out")).unwrap(),
        fs::read(f.path.join("fourteen")).unwrap()
    );
    f.no_staging();
}

#[test]
fn guest_diagnostics_are_distinct_from_job_failure_and_never_publish_a_program() {
    let f = Fixture::new();
    let output = f.run("diagnostic-compiler", "diagnostic-job", &[]);
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["execution"]["compiler_job"]["status"],
        "compile_error"
    );
    assert_eq!(
        report["execution"]["compiler_job"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let expected = fs::read(f.path.join("diagnostic-result")).unwrap();
    assert_eq!(fs::read(f.path.join("out")).unwrap(), expected);
    let output = f.run(
        "diagnostic-compiler",
        "diagnostic-job",
        &["--force", "--emit", "program"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("guest compilation failed: code6"));
    assert_eq!(fs::read(f.path.join("out")).unwrap(), expected);
    f.no_staging();
}

#[test]
fn all_failed_boundaries_preserve_previous_file_even_with_force() {
    let f = Fixture::new();
    fs::write(f.path.join("out"), b"previous").unwrap();
    for case in f.vectors["cases"].as_array().unwrap() {
        if let Some(message) = case["error"].as_str() {
            let output = f.run(
                case["program"].as_str().unwrap(),
                case["input"].as_str().unwrap(),
                &["--force"],
            );
            assert_eq!(output.status.code(), Some(1), "{case}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(message),
                "{case}"
            );
            assert_eq!(fs::read(f.path.join("out")).unwrap(), b"previous");
            f.no_staging();
        }
    }
    for args in [
        vec![],
        vec!["--force", "--modules", "1"],
        vec!["--force", "--validation-visits", "1"],
        vec!["--force", "--source-bytes", "0"],
        vec!["--force", "--sequence-length", "1048577"],
    ] {
        let output = f.run("compiler", "job", &args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(fs::read(f.path.join("out")).unwrap(), b"previous");
    }
    assert_eq!(
        f.run("generated", "zero", &["--force", "--emit", "program"])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(fs::read(f.path.join("out")).unwrap(), b"previous");
    f.no_staging();
}
