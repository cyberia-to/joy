use serde_json::Value;
#[path = "structured_certificates/deadline.rs"]
mod deadline;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "joy-certificate-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn raw(&self, name: &str) -> Value {
        let vectors: Vec<Value> =
            serde_json::from_str(include_str!("artifact_vectors.json")).unwrap();
        let v = vectors.into_iter().find(|v| v["name"] == name).unwrap();
        for field in ["program", "input", "expected_output"] {
            let bytes: Vec<u8> = serde_json::from_value(v[field].clone()).unwrap();
            fs::write(self.0.join(field), bytes).unwrap();
        }
        v
    }
    fn compiler(&self) {
        let vectors: Value = serde_json::from_str(include_str!("compiler_vectors.json")).unwrap();
        for (name, hex) in vectors["files"].as_object().unwrap() {
            let bytes: Vec<_> = hex
                .as_str()
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
                .collect();
            fs::write(self.0.join(name), bytes).unwrap();
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_joy"))
            .args(args)
            .current_dir(&self.0)
            .env("PATH", "")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("certificate command blocked");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.wait_with_output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let out = self.run(args);
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn fails(&self, args: &[&str]) {
        let out = self.run(args);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    fn clean(&self) {
        assert!(fs::read_dir(&self.0).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with('.')));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn raw_complete_nouns_and_loop_proofs_verify_in_fresh_processes() {
    for (name, cost) in [("add14", 3), ("identity_tree", 1), ("loop4097", 61_460)] {
        let f = Fixture::new();
        f.raw(name);
        let proved = f.ok(&[
            "prove-artifact",
            "program",
            "--input",
            "input",
            "-o",
            "proof",
        ]);
        let verified = f.ok(&[
            "verify-artifact",
            "program",
            "--input",
            "input",
            "--proof",
            "proof",
            "-o",
            "result",
        ]);
        assert_eq!(proved["schema"], "joy/artifact-proof/v1");
        assert_eq!(verified["schema"], "joy/artifact-verification/v1");
        assert_eq!(verified["verification"]["charged_reductions"], cost);
        for key in [
            "program_particle",
            "input_particle",
            "output_particle",
            "charged_reductions",
            "invocations",
            "logical_peak_frames",
            "expanded_steps",
            "semantic_events",
            "records",
            "transport",
        ] {
            assert_eq!(
                proved["verification"][key], verified["verification"][key],
                "{key}"
            );
        }
        assert_eq!(
            fs::read(f.0.join("result")).unwrap(),
            fs::read(f.0.join("expected_output")).unwrap()
        );
        assert_eq!(
            verified["verification"]["physical_resource_claim"],
            "unattested"
        );
        assert_eq!(verified["verification"]["prover_observations"], Value::Null);
        f.clean();
    }
}

#[test]
fn compiler_res1_success_diagnostics_and_extracted_artifact_are_bound() {
    let f = Fixture::new();
    f.compiler();
    for (program, job, result, status) in [
        ("compiler", "job", "result", "success"),
        (
            "diagnostic-compiler",
            "diagnostic-job",
            "diagnostic-result",
            "compile_error",
        ),
    ] {
        f.ok(&[
            "prove-artifact",
            program,
            "--input",
            job,
            "-o",
            "proof",
            "--force",
        ]);
        let v = f.ok(&[
            "verify-artifact",
            program,
            "--input",
            job,
            "--proof",
            "proof",
            "-o",
            "out",
            "--force",
        ]);
        assert_eq!(v["verification"]["compiler_job"]["status"], status);
        assert_eq!(
            fs::read(f.0.join("out")).unwrap(),
            fs::read(f.0.join(result)).unwrap()
        );
        if status == "success" {
            f.ok(&[
                "verify-artifact",
                program,
                "--input",
                job,
                "--proof",
                "proof",
                "-o",
                "compiled",
                "--emit",
                "program",
            ]);
            assert_eq!(
                fs::read(f.0.join("compiled")).unwrap(),
                fs::read(f.0.join("generated")).unwrap()
            );
            f.ok(&["run-artifact", "compiled", "--input", "zero", "-o", "run"]);
            assert_eq!(
                fs::read(f.0.join("run")).unwrap(),
                fs::read(f.0.join("fourteen")).unwrap()
            );
        } else {
            fs::write(f.0.join("compiled"), b"previous").unwrap();
            f.fails(&[
                "verify-artifact",
                program,
                "--input",
                job,
                "--proof",
                "proof",
                "-o",
                "compiled",
                "--emit",
                "program",
                "--force",
            ]);
            assert_eq!(fs::read(f.0.join("compiled")).unwrap(), b"previous");
        }
        let other = if program == "compiler" {
            "diagnostic-compiler"
        } else {
            "compiler"
        };
        f.fails(&["verify-artifact", other, "--input", job, "--proof", "proof"]);
        f.clean();
    }
}

#[test]
fn failed_proofs_and_verification_preserve_destinations_and_remove_prefixes() {
    let f = Fixture::new();
    f.raw("add14");
    fs::write(f.0.join("proof"), b"previous").unwrap();
    for extra in [
        vec![],
        vec!["--force", "--budget", "2"],
        vec!["--force", "--frames", "1"],
        vec!["--force", "--proof-bytes", "122"],
        vec!["--force", "--proof-records", "1"],
        vec!["--force", "--proof-steps", "1"],
        vec!["--force", "--proof-cache-slots", "0"],
    ] {
        let mut args = vec![
            "prove-artifact",
            "program",
            "--input",
            "input",
            "-o",
            "proof",
        ];
        args.extend(extra);
        f.fails(&args);
        assert_eq!(fs::read(f.0.join("proof")).unwrap(), b"previous");
        f.clean();
    }
    f.ok(&[
        "prove-artifact",
        "program",
        "--input",
        "input",
        "-o",
        "proof",
        "--force",
    ]);
    let good = fs::read(f.0.join("proof")).unwrap();
    fs::write(f.0.join("result"), b"previous").unwrap();
    for bytes in [
        good[..good.len() - 1].to_vec(),
        [good.clone(), vec![0]].concat(),
        {
            let mut bad = good.clone();
            bad[8] ^= 1;
            bad
        },
    ] {
        fs::write(f.0.join("bad"), bytes).unwrap();
        f.fails(&[
            "verify-artifact",
            "program",
            "--input",
            "input",
            "--proof",
            "bad",
            "-o",
            "result",
            "--force",
        ]);
        assert_eq!(fs::read(f.0.join("result")).unwrap(), b"previous");
        f.clean();
    }
    for extra in [
        vec!["--budget", "999999"],
        vec!["--proof-bytes", "122"],
        vec!["--proof-records", "1"],
        vec!["--proof-steps", "1"],
    ] {
        let mut args = vec![
            "verify-artifact",
            "program",
            "--input",
            "input",
            "--proof",
            "proof",
            "-o",
            "result",
            "--force",
        ];
        args.extend(extra);
        f.fails(&args);
        assert_eq!(fs::read(f.0.join("result")).unwrap(), b"previous");
        f.clean();
    }
    fs::write(
        f.0.join("input"),
        fs::read(f.0.join("expected_output")).unwrap(),
    )
    .unwrap();
    f.fails(&[
        "verify-artifact",
        "program",
        "--input",
        "input",
        "--proof",
        "proof",
    ]);
}

#[cfg(unix)]
#[test]
fn certificate_file_admission_rejects_links_and_pipes_without_blocking() {
    let f = Fixture::new();
    f.raw("add14");
    std::os::unix::fs::symlink("input", f.0.join("link")).unwrap();
    f.fails(&[
        "verify-artifact",
        "program",
        "--input",
        "input",
        "--proof",
        "link",
    ]);
    assert!(Command::new("mkfifo")
        .arg(f.0.join("pipe"))
        .status()
        .unwrap()
        .success());
    f.fails(&[
        "verify-artifact",
        "program",
        "--input",
        "input",
        "--proof",
        "pipe",
    ]);
}
