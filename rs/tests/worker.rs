use joy_rs::{
    worker::{verify_result, ExpectedJob, JobSpec, ProducedProof, ProofProfile, WorkerError},
    ExecutionArtifact, StateExecutionArtifact, Warrior, ZkExecutionArtifact,
};
use std::sync::OnceLock;
use trident::runtime::{artifact::BundleCost, ProgramBundle, ProgramInput};

struct Fixture {
    spec: JobSpec,
    bytes: Vec<u8>,
}
fn program(assembly: &str, state: bool) -> ProgramBundle {
    ProgramBundle {
        name: "worker_fixture".into(),
        version: "1".into(),
        target_vm: "nox".into(),
        target_os: Some("cyber".into()),
        source_hash: "fixture-source-identity".into(),
        assembly: assembly.into(),
        entry_point: "main".into(),
        functions: vec![],
        reads_state: state,
        cost: BundleCost {
            table_values: vec![],
            table_names: vec![],
            padded_height: 0,
            estimated_proving_ns: 0,
        },
    }
}
fn fixtures() -> &'static [Fixture; 4] {
    static FIXTURES: OnceLock<[Fixture; 4]> = OnceLock::new();
    FIXTURES.get_or_init(|| {
        let state = bbg::BbgState::new();
        let certificate =
            bbg::certificate::StateCertificate::from_state(&state, &(0..10).collect::<Vec<_>>())
                .unwrap();
        [
            ProofProfile::Public,
            ProofProfile::Private,
            ProofProfile::PublicState,
            ProofProfile::PrivateState,
        ]
        .map(|profile| {
            let stateful = matches!(
                profile,
                ProofProfile::PublicState | ProofProfile::PrivateState
            );
            let (assembly, secrets) = match profile {
                ProofProfile::Public => ("[5 [[0 2] [1 7]]]", vec![]),
                ProofProfile::Private => ("[5 [[0 2] [16 [[1 0] [1 0]]]]]", vec![7]),
                ProofProfile::PublicState => ("[17 [[1 0] [1 0]]]", vec![]),
                ProofProfile::PrivateState => ("[17 [[1 0] [16 [[1 0] [1 0]]]]]", vec![0]),
            };
            let bundle = program(assembly, stateful);
            let input = ProgramInput {
                public: vec![3],
                secret: secrets,
                digests: vec![],
            };
            let warrior = Warrior::with_budget(100);
            let (bytes, result) = match profile {
                ProofProfile::Public => {
                    let (a, r) = warrior.prove_execution(&bundle, &input, 100).unwrap();
                    (a.to_bytes().unwrap(), r)
                }
                ProofProfile::Private => {
                    let (a, r) = warrior.prove_zk_execution(&bundle, &input, 100).unwrap();
                    (a.to_bytes().unwrap(), r)
                }
                ProofProfile::PublicState => {
                    let (a, r) = warrior
                        .prove_state_certificate(&bundle, &input, &certificate, 100)
                        .unwrap();
                    (a.to_bytes().unwrap(), r)
                }
                ProofProfile::PrivateState => {
                    let (a, r) = warrior
                        .prove_zk_state_certificate(&bundle, &input, &certificate, 100)
                        .unwrap();
                    (a.to_bytes().unwrap(), r)
                }
            };
            Fixture {
                spec: JobSpec {
                    job_id: [42; 32],
                    attempt: 2,
                    profile,
                    bundle,
                    public_input: input.public,
                    expected_output: Some(result.output),
                    budget: 100,
                    max_artifact_bytes: bytes.len(),
                    expected_state_root: stateful.then(|| certificate.root().unwrap()),
                },
                bytes,
            }
        })
    })
}
fn result<'a>(f: &'a Fixture) -> ProducedProof<'a> {
    ProducedProof {
        job_id: f.spec.job_id,
        attempt: f.spec.attempt,
        profile: f.spec.profile,
        artifact: &f.bytes,
    }
}
fn check(f: &Fixture, spec: JobSpec) -> Result<joy_rs::worker::VerifiedExecution, WorkerError> {
    verify_result(&ExpectedJob::new(spec)?, result(f))
}

#[test]
fn all_profiles_verify_saved_computations_without_private_inputs() {
    for fixture in fixtures() {
        let job = ExpectedJob::new(fixture.spec.clone()).unwrap();
        let checked = verify_result(&job, result(fixture)).unwrap();
        assert_eq!(checked.job_id(), fixture.spec.job_id);
        assert_eq!(checked.attempt(), 2);
        assert_eq!(checked.profile(), fixture.spec.profile);
        assert_eq!(checked.public_input(), &[3]);
        assert_eq!(
            checked.public_output(),
            fixture.spec.expected_output.as_ref().unwrap()
        );
        assert_eq!(checked.state_root(), fixture.spec.expected_state_root);
        assert_eq!(checked.statement_budget(), 100);
        assert!(checked.reductions() > 0 && checked.reductions() <= 100);
        assert_eq!(
            checked.program_particle(),
            joy_rs::program_hash(&fixture.spec.bundle.assembly)
        );
        assert_eq!(job.spec().bundle.assembly, fixture.spec.bundle.assembly);
        let mut unpredicted = fixture.spec.clone();
        unpredicted.expected_output = None;
        assert_eq!(
            check(fixture, unpredicted).unwrap().public_output(),
            checked.public_output()
        );
    }
}

#[test]
fn dispatch_association_profile_and_byte_limits_precede_decoding() {
    for fixture in fixtures() {
        let job = ExpectedJob::new(fixture.spec.clone()).unwrap();
        let mut wrong = result(fixture);
        wrong.job_id[0] ^= 1;
        assert_eq!(
            verify_result(&job, wrong).unwrap_err(),
            WorkerError::JobMismatch
        );
        let mut wrong = result(fixture);
        wrong.attempt += 1;
        assert_eq!(
            verify_result(&job, wrong).unwrap_err(),
            WorkerError::AttemptMismatch
        );
        let mut wrong = result(fixture);
        wrong.profile = if fixture.spec.profile == ProofProfile::Private {
            ProofProfile::Public
        } else {
            ProofProfile::Private
        };
        assert_eq!(
            verify_result(&job, wrong).unwrap_err(),
            WorkerError::ProfileMismatch
        );
        let mut small = fixture.spec.clone();
        small.max_artifact_bytes -= 1;
        assert_eq!(
            check(fixture, small).unwrap_err(),
            WorkerError::ArtifactTooLarge
        );
        let mut retired = fixture.bytes.clone();
        retired[..8].copy_from_slice(b"JOYZK003");
        let mut wrong = result(fixture);
        wrong.artifact = &retired;
        assert_eq!(
            verify_result(&job, wrong).unwrap_err(),
            WorkerError::ProfileMismatch
        );
    }
}

#[test]
fn valid_proofs_for_different_saved_expectations_are_rejected() {
    for fixture in fixtures() {
        let mut other = fixture.spec.clone();
        other.public_input[0] += 1;
        assert_eq!(
            check(fixture, other).unwrap_err(),
            WorkerError::InputMismatch
        );
        let mut other = fixture.spec.clone();
        other.expected_output.as_mut().unwrap()[0] += 1;
        assert_eq!(
            check(fixture, other).unwrap_err(),
            WorkerError::OutputMismatch
        );
        let mut other = fixture.spec.clone();
        other.bundle.assembly = "[1 0]".into();
        assert_eq!(
            check(fixture, other).unwrap_err(),
            WorkerError::ProgramMismatch
        );
        let mut other = fixture.spec.clone();
        other.budget -= 1;
        assert_eq!(
            check(fixture, other).unwrap_err(),
            WorkerError::BudgetExceeded
        );
        if fixture.spec.expected_state_root.is_some() {
            let mut other = fixture.spec.clone();
            other.expected_state_root.as_mut().unwrap()[0] ^= 1;
            assert_eq!(
                check(fixture, other).unwrap_err(),
                WorkerError::StateMismatch
            );
        }
    }
}

#[test]
fn tampered_public_results_are_rejected_by_the_selected_native_verifier() {
    for fixture in fixtures() {
        let bytes = match fixture.spec.profile {
            ProofProfile::Public => {
                let mut a = ExecutionArtifact::from_bytes(&fixture.bytes).unwrap();
                a.statement.public_output[0] += 1;
                a.to_bytes().unwrap()
            }
            ProofProfile::PublicState => {
                let mut a = StateExecutionArtifact::from_bytes(&fixture.bytes).unwrap();
                a.statement.execution.public_output[0] += 1;
                a.to_bytes().unwrap()
            }
            ProofProfile::Private | ProofProfile::PrivateState => {
                let mut a = ZkExecutionArtifact::from_bytes(&fixture.bytes).unwrap();
                a.statement.execution.public_output[0] += 1;
                a.to_bytes().unwrap()
            }
        };
        let mut spec = fixture.spec.clone();
        spec.max_artifact_bytes += 1024;
        let job = ExpectedJob::new(spec).unwrap();
        let mut wrong = result(fixture);
        wrong.artifact = &bytes;
        assert_eq!(
            verify_result(&job, wrong).unwrap_err(),
            WorkerError::ProofRejected
        );
    }
}

#[test]
fn state_presence_cannot_be_downgraded_inside_the_shared_private_format() {
    let fixture = &fixtures()[3];
    let mut spec = fixture.spec.clone();
    spec.profile = ProofProfile::Private;
    spec.bundle.reads_state = false;
    spec.expected_state_root = None;
    let job = ExpectedJob::new(spec).unwrap();
    let mut wrong = result(fixture);
    wrong.profile = ProofProfile::Private;
    assert_eq!(
        verify_result(&job, wrong).unwrap_err(),
        WorkerError::ProfileMismatch
    );
}

#[test]
fn constructor_refuses_non_native_and_inconsistent_saved_jobs() {
    let fixture = &fixtures()[0];
    for change in 0..10 {
        let mut spec = fixture.spec.clone();
        match change {
            0 => spec.bundle.target_vm = "triton".into(),
            1 => spec.bundle.target_os = Some("neptune".into()),
            2 => spec.bundle.reads_state = true,
            3 => spec.expected_state_root = Some([0; 4]),
            4 => spec.public_input[0] = nebu::field::P,
            5 => spec.expected_output = Some(vec![nebu::field::P]),
            6 => spec.max_artifact_bytes = 0,
            7 => spec.budget = 0,
            8 => spec.max_artifact_bytes = 256 * 1024 * 1024 + 1,
            _ => spec.bundle.assembly = "malformed".into(),
        }
        assert!(ExpectedJob::new(spec).is_err(), "change {change}");
    }
    let mut state = fixtures()[2].spec.clone();
    state.expected_state_root = None;
    assert_eq!(
        ExpectedJob::new(state).unwrap_err(),
        WorkerError::InvalidJob
    );
}
