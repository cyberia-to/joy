use std::{fs, path::PathBuf, process::Command};

struct Installed(PathBuf);
impl Installed {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("joy-package-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        fs::copy(env!("CARGO_BIN_EXE_joy"), path.join("joy")).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(self.0.join("joy"))
            .args(args)
            .current_dir(&self.0)
            .env_remove("TRIDENT_STDLIB")
            .env_remove("TRIDENT_OSLIB")
            .env_remove("TRIDENT_EXTLIB")
            .output()
            .unwrap()
    }
}
impl Drop for Installed {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn installed_description_matches_machine_and_public_certificate_capabilities() {
    let installed = Installed::new();
    for target in ["nox", "cyber"] {
        let output = installed.run(&["describe", "--target", target]);
        assert!(output.status.success(), "{:?}", output);
        let package: trident::target::TargetPackage =
            serde_json::from_slice(&output.stdout).unwrap();
        package.validate().unwrap();
        assert_eq!(package.owner, "joy");
        assert_eq!(package.terrain.name, "nox");
        assert_eq!(package.terrain.digest_width, 4);
        assert_eq!(package.terrain.hash_rate, 8);
        assert_eq!(package.terrain.stack_depth, 0);
        assert!(package.runtime.run && package.runtime.prove && package.runtime.verify);
        assert!(!package.runtime.deploy);
        assert_eq!(package.runtime.proof_formats, [joy_rs::EXECUTION_FORMAT]);
        assert!(package
            .runtime
            .restrictions
            .iter()
            .any(|r| r.contains("neither zero knowledge")));
        assert!(package.union.is_none());
    }
    assert!(!installed
        .run(&["describe", "--target", "triton"])
        .status
        .success());
}

#[test]
fn installed_cli_rejects_state_instead_of_executing_statelessly() {
    let installed = Installed::new();
    fs::write(installed.0.join("quote.nox"), "[1 42]").unwrap();
    for args in [
        vec!["run", "quote.nox", "--state", "mainnet"],
        vec!["verify", "quote.nox", "--claim", "42", "--state", "mainnet"],
    ] {
        let output = installed.run(&args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("does not support --state"));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn installed_compiler_uses_embedded_target_constants() {
    let installed = Installed::new();
    fs::write(
        installed.0.join("main.tri"),
        "program widths\nuse std.target\nfn main() -> Field { target.DIGEST_WIDTH }\n",
    )
    .unwrap();
    let output = installed.run(&["run", "main.tri"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "4");
}

#[test]
fn stateless_library_runner_rejects_state_marked_bundle() {
    use trident::runtime::Runner;
    let mut bundle = trident::runtime::ProgramBundle::from_json(include_str!(
        "../../rs/tests/fixtures/add.bundle.json"
    ))
    .unwrap();
    bundle.reads_state = true;
    let error = joy_rs::Warrior::new()
        .run(
            &bundle,
            &trident::runtime::ProgramInput {
                public: vec![3, 5],
                secret: vec![],
                digests: vec![],
            },
        )
        .unwrap_err();
    assert!(error.contains("bundle requires state"));
}
