use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

struct Fixture {
    path: PathBuf,
    vector: Value,
}
impl Fixture {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "joy-structured-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let vectors: Vec<Value> =
            serde_json::from_str(include_str!("artifact_vectors.json")).unwrap();
        let vector = vectors.into_iter().find(|v| v["name"] == name).unwrap();
        let f = Self { path, vector };
        fs::write(f.path.join("program.dag"), f.bytes("program")).unwrap();
        fs::write(f.path.join("input.dag"), f.bytes("input")).unwrap();
        f
    }
    fn bytes(&self, key: &str) -> Vec<u8> {
        serde_json::from_value(self.vector[key].clone()).unwrap()
    }
    fn run(&self, extra: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_joy"))
            .args([
                "run-artifact",
                "program.dag",
                "--input",
                "input.dag",
                "--output",
                "out.dag",
            ])
            .args(extra)
            .current_dir(&self.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(10) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("structured command blocked");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.wait_with_output().unwrap()
    }
    fn ok(&self, extra: &[&str]) -> Value {
        let output = self.run(extra);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn fails(&self, extra: &[&str]) {
        let output = self.run(extra);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    fn no_staging(&self) {
        assert!(!fs::read_dir(&self.path).unwrap().any(|p| p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.')));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn cli_publishes_complete_nouns_with_exact_identity_and_execution_cost() {
    for (name, cost) in [("add14", 3), ("identity_tree", 1), ("loop4097", 61_460)] {
        let f = Fixture::new(name);
        let report = f.ok(&[]);
        assert_eq!(report["schema"], "joy/artifact-run/v1");
        assert_eq!(report["ok"], true);
        assert_eq!(report["artifact"], "out.dag");
        assert_eq!(report["execution"]["trace_mode"], "none");
        assert_eq!(report["execution"]["charged_reductions"], cost);
        assert_eq!(
            fs::read(f.path.join("out.dag")).unwrap(),
            f.bytes("expected_output")
        );
        for (vector, field) in [
            ("program", "program_particle"),
            ("input", "input_particle"),
            ("expected_output", "output_particle"),
        ] {
            let bytes = f.bytes(vector);
            let expected = bytes[8..40]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert_eq!(report["execution"][field], expected);
        }
        f.no_staging();
    }
}

#[test]
fn failures_preserve_destinations_and_never_publish_partial_success() {
    let f = Fixture::new("add14");
    f.fails(&["--budget", "2"]);
    assert!(!f.path.join("out.dag").exists());
    let report = f.ok(&["--budget", "3", "--frames", "2"]);
    let nodes = (report["execution"]["allocated_nodes"].as_u64().unwrap() - 1).to_string();
    fs::write(f.path.join("out.dag"), b"existing").unwrap();
    for args in [
        vec![],
        vec!["--force", "--budget", "2"],
        vec!["--force", "--frames", "1"],
        vec!["--force", "--arena-nodes", &nodes],
        vec!["--force", "--artifact-bytes", "1"],
        vec!["--force", "--time-ms", "0"],
    ] {
        f.fails(&args);
        assert_eq!(fs::read(f.path.join("out.dag")).unwrap(), b"existing");
        f.no_staging();
    }
    f.ok(&["--force"]);
    assert_eq!(
        fs::read(f.path.join("out.dag")).unwrap(),
        f.bytes("expected_output")
    );
    fs::remove_file(f.path.join("out.dag")).unwrap();
    fs::create_dir(f.path.join("out.dag")).unwrap();
    f.fails(&["--force"]);
    assert!(f.path.join("out.dag").is_dir());
    f.no_staging();
}

#[test]
fn invalid_input_has_no_published_output() {
    let f = Fixture::new("add14");
    fs::write(f.path.join("input.dag"), b"not an artifact").unwrap();
    f.fails(&[]);
    assert!(!f.path.join("out.dag").exists());
    #[cfg(unix)]
    {
        fs::remove_file(f.path.join("input.dag")).unwrap();
        assert!(Command::new("mkfifo")
            .arg(f.path.join("input.dag"))
            .status()
            .unwrap()
            .success());
        f.fails(&[]);
        fs::remove_file(f.path.join("input.dag")).unwrap();
        std::os::unix::fs::symlink(f.path.join("program.dag"), f.path.join("input.dag")).unwrap();
        f.fails(&[]);
    }
    f.no_staging();
}
