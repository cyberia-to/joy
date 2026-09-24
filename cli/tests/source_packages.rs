use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture {
    path: PathBuf,
    manifest: Value,
    expected: Vec<u8>,
}

fn unhex(value: &Value) -> Vec<u8> {
    value
        .as_str()
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "joy-pack-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("sources")).unwrap();
        let vectors: Value = serde_json::from_str(include_str!("compiler_vectors.json")).unwrap();
        for name in ["compiler", "generated", "zero", "fourteen"] {
            fs::write(path.join(name), unhex(&vectors["files"][name])).unwrap();
        }
        fs::write(path.join("sources/input.bytes"), b"raw\0\r\n\xff").unwrap();
        let manifest = json!({"version":1,"entry_module":"demo","entry_function":"main",
            "modules":[{"logical_path":"demo","file":"sources/input.bytes","origin_name":"fixture","origin_version":"1"}],
            "options":{"target":0,"input_profile":0,"output_profile":0,"optimization":0,"cfg_flags":["release"]},
            "limits":{"source_bytes":4096,"modules":32,"diagnostics":16,"sequence_length":4096,
                "validation_visits":100000,"artifact_bytes":1048576,"artifact_nodes":3000,
                "artifact_depth":128,"reductions":1000000,"arena_nodes":3000,"evaluator_frames":16384}});
        Self {
            path,
            manifest,
            expected: unhex(&vectors["files"]["job"]),
        }
    }
    fn pack(&self, manifest: &Value, extra: &[&str]) -> Output {
        fs::write(
            self.path.join("package.json"),
            serde_json::to_vec_pretty(manifest).unwrap(),
        )
        .unwrap();
        self.invoke(extra)
    }
    fn invoke(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .args(["pack-job", "--compiler"])
            .arg(self.path.join("compiler"))
            .arg("--manifest")
            .arg(self.path.join("package.json"))
            .arg("-o")
            .arg(self.path.join("job"))
            .args(extra)
            .output()
            .unwrap()
    }
    fn ok(&self, manifest: &Value) -> Value {
        let result = self.pack(manifest, &["--force"]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice(&result.stdout).unwrap()
    }
    fn failed(&self, manifest: &Value, extra: &[&str]) -> String {
        fs::write(self.path.join("job"), b"previous").unwrap();
        let result = self.pack(manifest, extra);
        assert_eq!(result.status.code(), Some(1), "{manifest}");
        assert!(result.stdout.is_empty());
        assert_eq!(fs::read(self.path.join("job")).unwrap(), b"previous");
        assert!(fs::read_dir(&self.path).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with('.')));
        String::from_utf8(result.stderr).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn packed_exact_source_bytes_match_independent_job_and_execute_on_the_guest() {
    let f = Fixture::new();
    let report = f.ok(&f.manifest);
    assert_eq!(report["schema"], "joy/job-pack/v1");
    assert_eq!(report["package"]["modules"][0]["source_bytes"], 7);
    assert_eq!(fs::read(f.path.join("job")).unwrap(), f.expected);
    let output = Command::new(env!("CARGO_BIN_EXE_joy"))
        .arg("run-artifact")
        .arg(f.path.join("compiler"))
        .arg("--input")
        .arg(f.path.join("job"))
        .args(["--emit", "program", "-o"])
        .arg(f.path.join("program"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let execution: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        execution["execution"]["input_particle"],
        report["package"]["job_particle"]
    );
    assert_eq!(
        fs::read(f.path.join("program")).unwrap(),
        fs::read(f.path.join("generated")).unwrap()
    );
}

#[test]
fn host_paths_order_and_timestamps_do_not_change_canonical_jobs() {
    let f = Fixture::new();
    let mut manifest = f.manifest.clone();
    manifest["modules"].as_array_mut().unwrap().push(json!({"logical_path":"unused", "file":"sources/unused", "origin_name":"dep", "origin_version":"v2"}));
    fs::write(f.path.join("sources/unused"), b"\0\xffignored module").unwrap();
    manifest["options"]["cfg_flags"] = json!(["release", "debug"]);
    let first = f.ok(&manifest);
    let bytes = fs::read(f.path.join("job")).unwrap();
    fs::rename(f.path.join("sources/input.bytes"), f.path.join("renamed")).unwrap();
    manifest["modules"][0]["file"] = json!(f.path.join("renamed"));
    manifest["modules"].as_array_mut().unwrap().reverse();
    manifest["options"]["cfg_flags"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(f.ok(&manifest)["package"], first["package"]);
    assert_eq!(fs::read(f.path.join("job")).unwrap(), bytes);
    fs::write(f.path.join("renamed"), b"raw\0\r\n\xfe").unwrap();
    assert_ne!(
        f.ok(&manifest)["package"]["job_particle"],
        first["package"]["job_particle"]
    );
    let previous = f.ok(&manifest)["package"]["job_particle"].clone();
    manifest["modules"][0]["origin_version"] = json!("v3");
    assert_ne!(f.ok(&manifest)["package"]["job_particle"], previous);
}

#[test]
fn unknown_missing_duplicate_and_unsupported_manifest_fields_reject_atomically() {
    let f = Fixture::new();
    for case in 0..15 {
        let mut manifest = f.manifest.clone();
        match case {
            0 => manifest["unknown"] = json!(1),
            1 => manifest["version"] = json!(2),
            2 => {
                manifest.as_object_mut().unwrap().remove("limits");
            }
            3 => manifest["modules"][0]["unknown"] = json!(1),
            4 => manifest["options"]["unknown"] = json!(1),
            5 => manifest["limits"]["unknown"] = json!(1),
            6 => {
                let duplicate = manifest["modules"][0].clone();
                manifest["modules"].as_array_mut().unwrap().push(duplicate);
            }
            7 => manifest["options"]["cfg_flags"] = json!(["release", "release"]),
            8 => manifest["options"]["target"] = json!(1),
            9 => manifest["options"]["output_profile"] = json!(1),
            10 => manifest["options"]["optimization"] = json!(1),
            11 => manifest["entry_module"] = json!("absent"),
            12 => manifest["modules"][0]["logical_path"] = json!("../escape"),
            13 => manifest["modules"][0]["origin_name"] = json!("\n"),
            _ => manifest["options"]["cfg_flags"] = json!(["a.b"]),
        }
        f.failed(&manifest, &["--force"]);
    }
    f.failed(&f.manifest, &[]);
}

#[test]
fn requested_and_independent_host_caps_bound_packaging() {
    let f = Fixture::new();
    let mut manifest = f.manifest.clone();
    manifest["limits"]["source_bytes"] = json!(7);
    f.ok(&manifest);
    for (field, value) in [
        ("source_bytes", 6),
        ("modules", 0),
        ("sequence_length", 0),
        ("validation_visits", 1),
        ("artifact_bytes", 1),
        ("artifact_nodes", 1),
        ("artifact_depth", 1),
        ("arena_nodes", 1),
        ("reductions", 1000001),
        ("evaluator_frames", 16385),
    ] {
        let mut manifest = f.manifest.clone();
        manifest["limits"][field] = json!(value);
        f.failed(&manifest, &["--force"]);
    }
    f.failed(&f.manifest, &["--force", "--source-bytes", "1"]);
    f.failed(&f.manifest, &["--force", "--time-ms", "0"]);
    // Invalid requests reject before any module file is opened.
    manifest["modules"][0]["file"] = json!("missing");
    manifest["limits"]["source_bytes"] = json!(0);
    assert!(f
        .failed(&manifest, &["--force"])
        .contains("unsupported job limit"));
}

#[test]
fn raw_compiler_and_nonregular_source_files_are_rejected_without_publication() {
    let f = Fixture::new();
    let compiler = fs::read(f.path.join("compiler")).unwrap();
    fs::copy(f.path.join("generated"), f.path.join("compiler")).unwrap();
    assert!(f
        .failed(&f.manifest, &["--force"])
        .contains("requires compiler profile"));
    fs::write(f.path.join("compiler"), compiler).unwrap();
    fs::remove_file(f.path.join("sources/input.bytes")).unwrap();
    fs::create_dir(f.path.join("sources/input.bytes")).unwrap();
    f.failed(&f.manifest, &["--force"]);
    fs::remove_dir(f.path.join("sources/input.bytes")).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(f.path.join("zero"), f.path.join("sources/input.bytes"))
            .unwrap();
        f.failed(&f.manifest, &["--force"]);
        fs::remove_file(f.path.join("sources/input.bytes")).unwrap();
        assert!(Command::new("mkfifo")
            .arg(f.path.join("sources/input.bytes"))
            .status()
            .unwrap()
            .success());
        f.failed(&f.manifest, &["--force"]);
    }
}

#[test]
fn duplicate_json_keys_are_rejected_without_last_value_wins() {
    let f = Fixture::new();
    let raw = serde_json::to_string(&f.manifest).unwrap();
    for needle in [
        "\"version\":1",
        "\"target\":0",
        "\"modules\":32",
        "\"origin_name\":\"fixture\"",
    ] {
        assert!(raw.contains(needle));
        let duplicate = raw.replacen(needle, &format!("{needle},{needle}"), 1);
        fs::write(f.path.join("package.json"), duplicate).unwrap();
        fs::write(f.path.join("job"), b"previous").unwrap();
        let result = f.invoke(&["--force"]);
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&result.stderr).contains("duplicate field"));
        assert!(result.stdout.is_empty());
        assert_eq!(fs::read(f.path.join("job")).unwrap(), b"previous");
    }
}

#[test]
fn aggregate_source_allowance_can_end_with_empty_files_but_cannot_reset() {
    let f = Fixture::new();
    let mut manifest = f.manifest.clone();
    fs::write(f.path.join("second"), b"1234567").unwrap();
    fs::write(f.path.join("empty"), b"").unwrap();
    for (name, file) in [("extra", "second"), ("zempty", "empty")] {
        manifest["modules"].as_array_mut().unwrap().push(
            json!({"logical_path":name,"file":file,"origin_name":"fixture","origin_version":"1"}),
        );
    }
    manifest["limits"]["source_bytes"] = json!(14);
    let report = f.ok(&manifest);
    assert_eq!(report["package"]["modules"][2]["source_bytes"], 0);
    manifest["limits"]["source_bytes"] = json!(13);
    f.failed(&manifest, &["--force"]);
    manifest["limits"]["source_bytes"] = json!(14);
    fs::write(f.path.join("empty"), b"x").unwrap();
    f.failed(&manifest, &["--force"]);
}
