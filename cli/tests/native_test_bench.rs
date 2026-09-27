use serde_json::Value;
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
            "joy-native-test-bench-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, source: &str) {
        fs::write(self.0.join(path), source).unwrap();
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
fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn test_command_executes_assertions_and_never_substitutes_main() {
    let f = Fixture::new();
    f.write("main.tri", "program checks\nfn main() { assert(false) }\n#[test]\nfn passes() { assert(3 + 5 == 8) }\n");
    let output = f.ok(&["test", "main.tri"]);
    assert!(text(&output).contains("checks.passes ... ok"));
    f.write("main.tri", "program checks\nfn main() {}\n#[test]\nfn fails() { assert(false) }\n#[test]\nfn passes() { assert(true) }\n");
    let output = f.bad(&["test", "main.tri"]);
    assert!(text(&output).contains("checks.fails ... FAILED"));
    assert!(text(&output).contains("checks.passes ... ok"));
    assert!(text(&output).contains("1 passed; 1 failed"));
}

#[test]
fn test_command_uses_project_profiles_and_imported_test_entries() {
    let f = Fixture::new();
    f.write("trident.toml", "[project]\nname = \"checks\"\nversion = \"0.1.0\"\nentry = \"main.tri\"\ntarget = \"nox\"\n[targets.special]\nflags = [\"special\"]\n");
    f.write("helper.tri", "module helper\npub fn value() -> Field { 7 }\n#[test]\nfn imported() { assert(value() == 7) }\n");
    f.write("main.tri", "program checks\nuse helper\nfn main() {}\n#[cfg(debug)]\n#[test]\nfn debug_ok() { assert(helper.value() == 7) }\n#[cfg(special)]\n#[test]\nfn special_bad() { assert(false) }\n#[cfg(test)]\n#[test]\nfn cfg_test() { assert(true) }\n");
    let output = f.ok(&["test", "."]);
    assert!(text(&output).contains("helper.imported ... ok"));
    assert!(text(&output).contains("3 passed; 0 failed"));
    let output = f.bad(&["test", ".", "--profile", "special"]);
    assert!(text(&output).contains("checks.special_bad ... FAILED"));
    assert!(text(&output).contains("2 passed; 1 failed"));
    f.bad(&["test", ".", "--target", "triton"]);
}

#[test]
fn bench_reports_actual_native_execution_counts_and_independent_expectations() {
    let f = Fixture::new();
    f.write(
        "main.tri",
        "program measure\nfn main(x: Field) -> Field { x * x + 1 }\n",
    );
    let run = f.ok(&["run", "main.tri", "--input-values", "6"]);
    assert_eq!(run.stdout, b"37\n");
    let count: u64 = String::from_utf8_lossy(&run.stderr)
        .split_whitespace()
        .find_map(|s| s.parse().ok())
        .unwrap();
    let measured = f.ok(&[
        "bench",
        "main.tri",
        "--input-values",
        "6",
        "--claim",
        "37",
        "--repeat",
        "3",
    ]);
    let report: Value = serde_json::from_slice(&measured.stdout).unwrap();
    assert_eq!(report["schema"], "joy/bench/v1");
    assert_eq!(report["reductions"], count.to_string());
    assert_eq!(report["output"], serde_json::json!(["37"]));
    assert_eq!(report["expected_output_checked"], true);
    assert_eq!(report["proof_generated"], false);
    let samples = report["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 3);
    let mut times = Vec::new();
    for sample in samples {
        assert_eq!(sample["reductions"], count.to_string());
        times.push(
            sample["elapsed_ns"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
        );
    }
    assert_eq!(report["total_ns"], times.iter().sum::<u128>().to_string());
    times.sort_unstable();
    assert_eq!(report["min_ns"], times[0].to_string());
    assert_eq!(report["median_ns"], times[1].to_string());
    f.bad(&["bench", "main.tri", "--input-values", "6", "--claim", "38"]);
    f.bad(&["bench", "main.tri", "--input-values", "6", "--budget", "1"]);
    for n in ["0", "1001"] {
        f.bad(&["bench", "main.tri", "--repeat", n]);
    }
}

#[test]
fn bench_restarts_private_input_stream_and_accepts_exported_bundles() {
    let f = Fixture::new();
    f.write(
        "main.tri",
        "program private_measure\nfn main(x: Field) -> Field { let y: Field = divine()\n x + y }\n",
    );
    f.write(
        "input.json",
        r#"{"schema_version":1,"public":["17"],"secret":["25"]}"#,
    );
    f.ok(&["build", "main.tri", "--output", "main.bundle.json"]);
    let output = f.ok(&[
        "bench",
        "main.bundle.json",
        "--input-file",
        "input.json",
        "--claim",
        "42",
        "--repeat",
        "2",
    ]);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["output"], serde_json::json!(["42"]));
    assert_eq!(report["samples"].as_array().unwrap().len(), 2);
    assert!(report.get("secret").is_none());
    f.write("raw.nox", "[1 24]");
    let output = f.ok(&["bench", "raw.nox", "--repeat", "1"]);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["output"], serde_json::json!(["24"]));
    assert_eq!(report["expected_output_checked"], false);
    assert_eq!(report["reductions"], "1");
}

#[test]
fn bench_state_execution_reports_the_authenticated_root_and_rejects_tampering() {
    let f = Fixture::new();
    f.write(
        "state.tri",
        "program measure_state\nfn main(k: Field) -> Field { os.state.read(k) + 5 }\n",
    );
    let mut state = bbg::BbgState::new();
    let mut record = bbg::types::ParticleRecord::zero();
    record.energy = 77;
    state.particles.insert([1; 32], record);
    let certificate = bbg::certificate::StateCertificate::from_state(&state, &[0]).unwrap();
    fs::write(
        f.0.join("state.json"),
        serde_json::to_vec(&certificate).unwrap(),
    )
    .unwrap();
    let output = f.ok(&[
        "bench",
        "state.tri",
        "--state",
        "state.json",
        "--input-values",
        "11",
        "--claim",
        "82",
        "--repeat",
        "2",
    ]);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["state_root"],
        serde_json::json!(certificate.root().unwrap().map(|v| v.to_string()))
    );
    let mut wrong = certificate;
    wrong.dimensions[0].fields[11] += 1;
    fs::write(f.0.join("wrong.json"), serde_json::to_vec(&wrong).unwrap()).unwrap();
    let output = f.bad(&[
        "bench",
        "state.tri",
        "--state",
        "wrong.json",
        "--input-values",
        "11",
    ]);
    assert!(output.stdout.is_empty());
}
