use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
use trident::runtime::ProgramBundle;

struct Fixture(PathBuf);
impl Fixture {
    fn new(target: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "joy-build-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("src")).unwrap();
        let f = Self(path);
        f.write("trident.toml",&format!("[project]\nname = \"fleet\"\nversion = \"1.2.3\"\nentry = \"src/main.tri\"\ntarget = \"{target}\"\n[targets.production]\nflags = [\"tuned\"]\n"));
        f.write("src/main.tri","program application\nuse helper\n#[cfg(debug)]\nfn main(x: Field) -> Field { helper.calculate(x) + 1 }\n#[cfg(release)]\nfn main(x: Field) -> Field { helper.calculate(x) + 2 }\n#[cfg(tuned)]\nfn main(x: Field) -> Field { helper.calculate(x) + 9 }\n");
        f.write("src/helper.tri","module helper\nfn hidden(x: Field) -> Field { x * 7 }\npub fn calculate(x: Field) -> Field { hidden(x) }\n");
        f
    }
    fn write(&self, path: &str, value: &str) {
        fs::write(self.0.join(path), value).unwrap();
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
            "{args:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
    fn bad(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{args:?}\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        output
    }
    fn bundle(&self, path: &str) -> ProgramBundle {
        ProgramBundle::from_json(&fs::read_to_string(self.0.join(path)).unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn project_build_run_prove_verify_preserves_custom_profile_and_imports() {
    let f = Fixture::new("nox");
    f.ok(&["build", ".", "--profile", "production"]);
    assert!(f.0.join("fleet.bundle.json").is_file());
    assert!(!f.0.join("main.bundle.json").exists());
    let bundle = f.bundle("fleet.bundle.json");
    assert_eq!(bundle.name, "fleet");
    assert_eq!(bundle.version, "1.2.3");
    assert_eq!(bundle.target_vm, "nox");
    assert!(!bundle.reads_state);
    assert!(!bundle.entry_point.is_empty());
    assert!(!bundle.functions.is_empty());
    for input in [".", "src/main.tri", "fleet.bundle.json"] {
        let output = f.ok(&[
            "run",
            input,
            "--profile",
            "production",
            "--input-values",
            "5",
        ]);
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "44");
    }
    f.ok(&[
        "prove",
        "fleet.bundle.json",
        "--input-values",
        "5",
        "--output",
        "built.zheng",
    ]);
    f.ok(&[
        "verify",
        "src/main.tri",
        "--profile",
        "production",
        "--proof",
        "built.zheng",
        "--claim",
        "44",
    ]);
    f.ok(&[
        "prove",
        ".",
        "--profile",
        "production",
        "--input-values",
        "5",
        "--output",
        "source.zheng",
    ]);
    f.ok(&[
        "verify",
        "fleet.bundle.json",
        "--proof",
        "source.zheng",
        "--claim",
        "44",
    ]);
    f.bad(&["verify", "built.zheng", "--claim", "45"]);
    f.write(
        "src/helper.tri",
        "module helper\npub fn calculate(x: Field) -> Field { x * 8 }\n",
    );
    f.bad(&[
        "verify",
        ".",
        "--profile",
        "production",
        "--proof",
        "built.zheng",
        "--claim",
        "44",
    ]);
}

#[test]
fn explicit_target_overrides_project_for_build_run_and_prove() {
    let f = Fixture::new("triton");
    for command in ["build", "run", "prove"] {
        f.bad(&[command, "."]);
    }
    assert!(!f.0.join("fleet.bundle.json").exists());
    f.ok(&["build", ".", "--target", "cyber"]);
    let output = f.ok(&["run", ".", "--target", "nox", "--input-values", "5"]);
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "36");
    f.ok(&[
        "prove",
        ".",
        "--target",
        "nox",
        "--input-values",
        "5",
        "--output",
        "override.zheng",
    ]);
    f.ok(&["verify", "override.zheng", "--claim", "36"]);
    f.bad(&["build", ".", "--target", "unknown", "--force"]);
    assert_eq!(f.bundle("fleet.bundle.json").target_vm, "nox");
}

#[test]
fn source_default_name_nox_export_and_checkout_independent_bundles() {
    let f = Fixture::new("nox");
    let copy = Fixture::new("nox");
    f.ok(&["build", "src/main.tri", "--profile", "release"]);
    f.ok(&[
        "build",
        "src/main.tri",
        "--profile",
        "release",
        "--emit",
        "nox",
    ]);
    let bundle = f.bundle("src/main.bundle.json");
    assert_eq!(
        fs::read_to_string(f.0.join("src/main.nox")).unwrap(),
        bundle.assembly
    );
    let run = f.ok(&["run", "src/main.nox", "--input-values", "5"]);
    assert_eq!(String::from_utf8(run.stdout).unwrap().trim(), "37");
    for fixture in [&f, &copy] {
        fixture.ok(&[
            "build",
            ".",
            "--profile",
            "production",
            "--output",
            "portable.json",
        ]);
    }
    assert_eq!(
        fs::read(f.0.join("portable.json")).unwrap(),
        fs::read(copy.0.join("portable.json")).unwrap()
    );
}

#[test]
fn publication_requires_force_and_failures_preserve_existing_artifact() {
    let f = Fixture::new("nox");
    f.ok(&["build", ".", "-o", "artifact.json"]);
    let original = fs::read(f.0.join("artifact.json")).unwrap();
    f.bad(&["build", ".", "-o", "artifact.json", "--profile", "release"]);
    assert_eq!(fs::read(f.0.join("artifact.json")).unwrap(), original);
    f.ok(&[
        "build",
        ".",
        "-o",
        "artifact.json",
        "--profile",
        "release",
        "--force",
    ]);
    let replaced = fs::read(f.0.join("artifact.json")).unwrap();
    assert_ne!(replaced, original);
    for command in ["build", "run", "prove"] {
        f.bad(&[command, ".", "--profile", "misspelled"]);
    }
    f.write(
        "src/main.tri",
        "program broken\nfn main() { undefined() }\n",
    );
    f.bad(&["build", ".", "-o", "artifact.json", "--force"]);
    assert_eq!(fs::read(f.0.join("artifact.json")).unwrap(), replaced);
    assert!(!fs::read_dir(&f.0).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".joy-build-")));
}

#[test]
fn json_v1_reports_exact_result_identity_and_structured_failure() {
    let f = Fixture::new("nox");
    let output = f.ok(&[
        "build",
        ".",
        "--profile",
        "production",
        "--format",
        "json-v1",
    ]);
    assert_eq!(output.stdout.iter().filter(|&&b| b == b'\n').count(), 1);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "joy/cli/v1");
    assert_eq!(value["command"], "build");
    assert_eq!(value["ok"], true);
    assert!(value["error"].is_null());
    let result = &value["result"];
    assert_eq!(result["kind"], "build");
    assert_eq!(result["format"], "bundle");
    assert_eq!(result["program"], "fleet");
    assert_eq!(result["profile"], "production");
    assert_eq!(
        result["source_hash"],
        f.bundle("fleet.bundle.json").source_hash
    );
    assert_eq!(result["compiler"]["version"], trident::COMPILER_VERSION);
    assert_eq!(result["compiler"]["api"], trident::COMPILER_API);
    assert_eq!(
        result["target_package"]["compilation_hash"],
        joy_rs::target_package("nox")
            .unwrap()
            .compilation_hash()
            .unwrap()
    );
    let failed = f.bad(&["build", ".", "--format", "json-v1"]);
    assert_eq!(failed.stdout.iter().filter(|&&b| b == b'\n').count(), 1);
    let error: serde_json::Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(error["ok"], false);
    assert!(error["result"].is_null());
    assert_eq!(error["error"]["code"], "artifact_write_failed");
    assert_eq!(error["error"]["retryable"], false);
}
