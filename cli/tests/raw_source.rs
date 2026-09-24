//! Real SDK imports -> seed ART1 compiler -> Joy nox evaluator -> canonical result.
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "joy-raw-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(p.join("src")).unwrap();
        let p = Self(p);
        p.write("trident.toml", "[project]\nname = \"native\"\nversion = \"0.1.0\"\nentry = \"src/main.tri\"\ntarget = \"nox\"\n[targets.copy]\nflags = [\"copy\"]\n");
        p.write("src/main.tri", "program source\nuse vm.nox.noun\nuse helper\n#[cfg(debug)]\nfn main(input: Noun) -> Noun { helper.add(input) }\n#[cfg(copy)]\nfn main(input: Noun) -> Noun { input }\n");
        p.write("src/helper.tri", "module helper\nuse vm.nox.noun\n#[pure]\npub fn add(input: Noun) -> Noun { noun.atom(noun.as_field(input) + 7 + 7) }\n");
        p
    }
    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_joy"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn bad(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        assert_eq!(output.status.code(), Some(1));
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn fixture(&self, name: &str) -> Vec<u8> {
        let vectors: Vec<Value> =
            serde_json::from_str(include_str!("artifact_vectors.json")).unwrap();
        let v = vectors.into_iter().find(|v| v["name"] == name).unwrap();
        let input: Vec<u8> = serde_json::from_value(v["input"].clone()).unwrap();
        fs::write(self.0.join("input.dag"), input).unwrap();
        serde_json::from_value(v["expected_output"].clone()).unwrap()
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn imported_source_compiles_to_art1_and_joy_executes_the_complete_output() {
    let p = Project::new();
    let build = p.ok(&["build", ".", "--emit", "artifact", "--format", "json-v1"]);
    assert_eq!(build["result"]["format"], "artifact");
    assert_eq!(build["result"]["program"], "native");
    assert_eq!(build["result"]["input_profile"], 0);
    let expected = p.fixture("add14");
    let execution = p.ok(&[
        "run-artifact",
        "native.dag",
        "--input",
        "input.dag",
        "-o",
        "out.dag",
    ]);
    assert_eq!(fs::read(p.0.join("out.dag")).unwrap(), expected);
    assert_eq!(
        execution["execution"]["program_particle"],
        build["result"]["program_particle"]
    );
    let file_build = p.ok(&[
        "build",
        "src/main.tri",
        "--emit",
        "artifact",
        "--format",
        "json-v1",
    ]);
    assert_eq!(
        fs::read(p.0.join("src/main.dag")).unwrap(),
        fs::read(p.0.join("native.dag")).unwrap()
    );
    assert_eq!(
        file_build["result"]["program_particle"],
        build["result"]["program_particle"]
    );
    let api = trident::compile_raw_artifact_project(
        &p.0.join("src/main.tri"),
        &trident::CompileOptions::default()
            .with_package(joy_rs::target_package("nox").unwrap())
            .unwrap(),
        trident::RAW_ARTIFACT_LIMITS,
    )
    .unwrap();
    assert_eq!(api.bytes, fs::read(p.0.join("native.dag")).unwrap());
}

#[test]
fn explicit_compiler_profile_export_matches_seed_api_and_refuses_other_formats() {
    let p = Project::new();
    p.write(
        "src/main.tri",
        include_str!("../../rs/tests/fixtures/compiler_transport.tri"),
    );
    let build = p.ok(&[
        "build",
        ".",
        "--emit",
        "artifact",
        "--artifact-profile",
        "compiler-job",
        "--format",
        "json-v1",
    ]);
    assert_eq!(build["result"]["input_profile"], 1);
    assert_eq!(build["result"]["output_profile"], 1);
    let bytes = fs::read(p.0.join("native.dag")).unwrap();
    let api = trident::compile_native_artifact_project(
        &p.0.join("src/main.tri"),
        &trident::CompileOptions::default()
            .with_package(joy_rs::target_package("nox").unwrap())
            .unwrap(),
        trident::NativeArtifactProfile::CompilerJob,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .unwrap();
    assert_eq!(api.bytes, bytes);
    for emit in ["bundle", "nox"] {
        let error = p.bad(&[
            "build",
            ".",
            "--emit",
            emit,
            "--artifact-profile",
            "compiler-job",
            "--format",
            "json-v1",
            "--force",
            "-o",
            "native.dag",
        ]);
        assert!(error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("requires --emit artifact"));
        assert_eq!(fs::read(p.0.join("native.dag")).unwrap(), bytes);
    }
    // Profile1 does not turn arbitrary nouns into admitted jobs.
    p.fixture("add14");
    let output = p.run(&[
        "run-artifact",
        "native.dag",
        "--input",
        "input.dag",
        "-o",
        "out.dag",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("compiler job admission"));
    assert!(!p.0.join("out.dag").exists());
}

#[test]
fn named_profile_and_target_override_preserve_exact_nested_input() {
    let p = Project::new();
    p.write("trident.toml", "[project]\nname = \"native\"\nversion = \"0.1.0\"\nentry = \"src/main.tri\"\ntarget = \"triton\"\n[targets.copy]\nflags = [\"copy\"]\n");
    p.bad(&[
        "build",
        ".",
        "--emit",
        "artifact",
        "--profile",
        "copy",
        "--format",
        "json-v1",
    ]);
    p.ok(&[
        "build",
        ".",
        "--emit",
        "artifact",
        "--profile",
        "copy",
        "--target",
        "nox",
        "--format",
        "json-v1",
    ]);
    let expected = p.fixture("identity_tree");
    p.ok(&[
        "run-artifact",
        "native.dag",
        "--input",
        "input.dag",
        "-o",
        "out.dag",
    ]);
    assert_eq!(fs::read(p.0.join("out.dag")).unwrap(), expected);
}

#[test]
fn artifact_publication_preserves_existing_output_on_every_compile_failure() {
    let p = Project::new();
    let args = ["build", ".", "--emit", "artifact", "--format", "json-v1"];
    p.ok(&args);
    let saved = fs::read(p.0.join("native.dag")).unwrap();
    let refused = p.bad(&args);
    assert_eq!(refused["error"]["code"], "artifact_write_failed");
    for source in [
        "program p\nfn main(x: Field) -> Field { x }",
        "program p\nfn main(input: Noun) -> Noun { nox_noun_atom(divine()) }",
        "program p\nfn main(input: Noun) -> Noun { input == input }",
    ] {
        p.write("src/main.tri", source);
        let failed = p.bad(&[
            "build", ".", "--emit", "artifact", "--format", "json-v1", "--force",
        ]);
        assert_eq!(failed["error"]["code"], "compile_failed");
        assert_eq!(fs::read(p.0.join("native.dag")).unwrap(), saved);
    }
    assert!(fs::read_dir(&p.0).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with('.')));
}

#[test]
fn compact_source_loop_uses_explicit_frames_and_never_publishes_partial_execution() {
    let p = Project::new();
    p.write("src/main.tri", "program loop_test\nfn increment(x: Field) -> Field { x + 1 }\nfn main(input: Noun) -> Noun { let mut total: Field = 0\nfor i in 0..5000 { total = increment(total) }\nnox_noun_atom(total) }");
    p.fixture("add14");
    let build = p.ok(&["build", ".", "--emit", "artifact", "--format", "json-v1"]);
    assert!(fs::metadata(p.0.join("native.dag")).unwrap().len() < 30_000);
    p.write("out.dag", "previous successful result");
    let args = [
        "run-artifact",
        "native.dag",
        "--input",
        "input.dag",
        "-o",
        "out.dag",
        "--force",
    ];
    let error = p.run(&args);
    assert_eq!(error.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&error.stderr).contains("Frames"),
        "{error:?}"
    );
    assert_eq!(
        fs::read(p.0.join("out.dag")).unwrap(),
        b"previous successful result"
    );
    for extra in [
        ["--frames", "65536", "--budget", "1"],
        ["--frames", "65536", "--arena-nodes", "1000"],
    ] {
        let mut limited = args.to_vec();
        limited.extend(extra);
        assert_eq!(p.run(&limited).status.code(), Some(1));
        assert_eq!(
            fs::read(p.0.join("out.dag")).unwrap(),
            b"previous successful result"
        );
    }
    let mut admitted = args.to_vec();
    admitted.extend(["--frames", "65536"]);
    let execution = p.ok(&admitted);
    assert_eq!(
        execution["execution"]["program_particle"],
        build["result"]["program_particle"]
    );
    assert!(execution["execution"]["peak_frames"].as_u64().unwrap() > 16384);
    assert_eq!(execution["execution"]["trace_mode"], "none");
    let output = fs::read(p.0.join("out.dag")).unwrap();
    // Canonical one-atom NOXDAG01, independently interpreted as integer 5000.
    assert_eq!(output.len(), 85);
    assert_eq!(&output[..8], b"NOXDAG01");
    assert_eq!(&output[40..44], &1u32.to_le_bytes());
    assert_eq!(&output[8..40], &output[44..76]);
    assert_eq!(output[76], 8);
    assert_eq!(&output[77..85], &5000u64.to_le_bytes());
}
