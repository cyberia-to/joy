//! Public words must satisfy the source signature inside the proven computation.
use joy_rs::{ExecutionArtifact, Warrior, ZkExecutionArtifact};
use trident::runtime::{ProgramBundle, ProgramInput, Runner};

static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn compile(source: &str, profile: &str) -> ProgramBundle {
    let directory = std::env::temp_dir().join(format!(
        "joy-typed-entry-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("entry.tri");
    std::fs::write(&path, source).unwrap();
    let bundle =
        trident::compile_to_bundle(&path, &trident::CompileOptions::for_profile(profile)).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    bundle
}
fn input(public: &[u64]) -> ProgramInput {
    ProgramInput {
        public: public.to_vec(),
        secret: vec![],
        digests: vec![],
    }
}

#[test]
fn unused_typed_inputs_are_constrained_by_public_execution_certificate() {
    let source = "program entry\nfn main(a: Field,n: U32,flag: Bool)->Field { 11 }";
    let warrior = Warrior::new();
    for profile in ["debug", "release"] {
        let bundle = compile(source, profile);
        let values = [7, u32::MAX as u64, 0];
        let (artifact, native) = warrior
            .prove_execution(&bundle, &input(&values), 100_000)
            .unwrap();
        assert_eq!(native.output, vec![11]);
        let decoded = ExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
        decoded.verify().unwrap();
        for public in [
            vec![],
            vec![7, 19],
            vec![7, 19, 0, 31],
            vec![7, 1u64 << 32, 0],
            vec![7, 19, 2],
        ] {
            assert!(warrior.run(&bundle, &input(&public)).is_err());
            assert!(
                warrior
                    .prove_execution(&bundle, &input(&public), 100_000)
                    .is_err()
            );
            // Same constant output cannot make a mistyped/mis-sized input valid.
            let mut changed = decoded.clone();
            changed.statement.public_input = public;
            assert!(changed.verify().is_err());
        }
    }
}

#[test]
fn aggregate_entry_has_real_proven_native_layout() {
    let source = "program entry\nstruct Inner { n: U32, flag: Bool }\nstruct Outer { a: Field, inner: Inner }\nfn main(x: Outer,words:[Field;3],digest:Digest)->Field { assert(x.inner.flag)\n assert(words[0] == 101)\n assert(words[1] == 103)\n assert(words[2] == 107)\n assert(digest[0] == 307)\n assert(digest[1] == 311)\n assert(digest[2] == 313)\n assert(digest[3] == 317)\n x.a*100+as_field(x.inner.n) }";
    let values = [7, 19, 0, 101, 103, 107, 307, 311, 313, 317];
    for profile in ["debug", "release"] {
        let bundle = compile(source, profile);
        let (artifact, native) = Warrior::new()
            .prove_execution(&bundle, &input(&values), 100_000)
            .unwrap();
        assert_eq!(native.output, vec![719]);
        let decoded = ExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
        decoded.verify().unwrap();
        for i in 0..values.len() {
            let mut changed = decoded.clone();
            changed.statement.public_input[i] += 1;
            assert!(changed.verify().is_err(), "input leaf {i} was not bound");
        }
        let mut changed = decoded;
        changed.statement.public_output[0] = 718;
        assert!(changed.verify().is_err());
    }
}

#[test]
fn typed_input_guards_survive_the_real_private_stark() {
    let bundle = compile(
        "program entry\nfn main(n:U32,flag:Bool)->Field { assert(flag)\n let secret:Field=divine()\n as_field(n)+secret }",
        "release",
    );
    let mut values = input(&[19, 0]);
    values.secret = vec![23];
    let (artifact, native) = Warrior::new()
        .prove_zk_execution(&bundle, &values, 100_000)
        .unwrap();
    assert_eq!(native.output, vec![42]);
    let decoded = ZkExecutionArtifact::from_bytes(&artifact.to_bytes().unwrap()).unwrap();
    decoded.verify().unwrap();
    for public in [vec![1u64 << 32, 0], vec![19, 2], vec![19], vec![19, 0, 7]] {
        let mut changed = decoded.clone();
        changed.statement.execution.public_input = public;
        assert!(changed.verify().is_err());
    }
    let mut changed = decoded;
    changed.statement.execution.public_output[0] = 43;
    assert!(changed.verify().is_err());
}
