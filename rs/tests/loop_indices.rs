//! Exact FINAL4 installed-smoke sources, preserved as regression fixtures.
use joy_rs::{ExecutionArtifact, Warrior, ZkExecutionArtifact};
use trident::runtime::ProgramInput;

const HELPER: &str = r#"module release_helper
const OFFSET:Field=18446744069414584328
pub struct Pair { a:Field,b:Field }
fn add(x:Field)->Field {x+OFFSET}
pub fn fold<N>(words:[Field;N])->Field {let mut value:Field=0
for i in 0..N {value=value*10+words[i]}
if value==35 {add(value)} else {value}}
pub fn pair()->Pair {Pair {b:19,a:7}}
"#;
const ENTRY: &str = r#"program imported
use release_helper
const OFFSET:Field=1000
fn main(words:[Field;2])->Field {let p=release_helper.pair()
release_helper.fold<2>(words)+release_helper.fold(words)+p.a*100+p.b}
"#;

struct Project(std::path::PathBuf);
impl Project {
    fn new(label: &str) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("joy-{label}-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn entry(&self, source: &str) -> std::path::PathBuf {
        std::fs::write(self.0.join("release_helper.tri"), HELPER).unwrap();
        let path = self.0.join("entry.tri");
        std::fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn exact_final4_imported_loop_indices_have_public_and_private_proofs() {
    let project = Project::new("final4-loop-indices");
    let path = project.entry(ENTRY);
    let warrior = Warrior::new();
    for profile in ["debug", "release"] {
        let bundle =
            trident::compile_to_bundle(&path, &trident::CompileOptions::for_profile(profile))
                .unwrap();
        for (first, expected) in [(3, 803), (4, 809)] {
            let input = ProgramInput {
                public: vec![first, 5],
                secret: vec![],
                digests: vec![],
            };
            let (public, native) = warrior.prove_execution(&bundle, &input, 100_000).unwrap();
            assert_eq!(native.output, [expected]);
            let public = ExecutionArtifact::from_bytes(&public.to_bytes().unwrap()).unwrap();
            public.verify().unwrap();
            for output in [false, true] {
                let mut bad = public.clone();
                if output {
                    bad.statement.public_output[0] += 1;
                } else {
                    bad.statement.public_input[0] += 1;
                }
                assert!(
                    bad.verify().is_err(),
                    "public mutation accepted: {profile}, {first}, {output}"
                );
            }
            let (private, native) = warrior
                .prove_zk_execution(&bundle, &input, 100_000)
                .unwrap();
            assert_eq!(native.output, [expected]);
            let private = ZkExecutionArtifact::from_bytes(&private.to_bytes().unwrap()).unwrap();
            private.verify().unwrap();
            for output in [false, true] {
                let mut bad = private.clone();
                if output {
                    bad.statement.execution.public_output[0] += 1;
                } else {
                    bad.statement.execution.public_input[0] += 1;
                }
                assert!(
                    bad.verify().is_err(),
                    "private mutation accepted: {profile}, {first}, {output}"
                );
            }
        }
    }
}

#[test]
fn loop_index_constants_do_not_escape_shadowed_or_returning_frames() {
    let project = Project::new("loop-index-scopes");
    let warrior = Warrior::new();
    let source="program scopes\nconst i:U32=2\nfn main(flag:Field)->Field{let words=[7,19,31]\nlet mut total:Field=0\nfor i in 0..2 {if flag==0 {return words[i]} else {let i=as_u32(flag)}\nfor i in 0..2 {total=total+words[i]}\ntotal=total+words[i]}\ntotal+words[i]}";
    let path = project.entry(source);
    for profile in ["debug", "release"] {
        let bundle =
            trident::compile_to_bundle(&path, &trident::CompileOptions::for_profile(profile))
                .unwrap();
        for (flag, expected) in [(0, 7), (1, 109)] {
            let input = ProgramInput {
                public: vec![flag],
                secret: vec![],
                digests: vec![],
            };
            let (certificate, native) = warrior.prove_execution(&bundle, &input, 100_000).unwrap();
            assert_eq!(native.output, [expected]);
            certificate.verify().unwrap();
        }
    }
    // The nearer runtime variable must prevent fallback to the module/loop constant.
    let invalid = source.replace(
        "total=total+words[i]}",
        "let i=as_u32(flag)\ntotal=total+words[i]}",
    );
    let error = trident::compile_to_bundle(
        &project.entry(&invalid),
        &trident::CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        error.iter().any(|e| e
            .message
            .contains("array index must be a compile-time constant")),
        "{error:?}"
    );
}
