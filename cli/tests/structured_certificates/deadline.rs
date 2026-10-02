use super::*;

#[test]
fn compiler_certificate_bytes_and_claim_survive_independent_host_deadlines() {
    let f = Fixture::new();
    f.compiler();
    let compact = [
        "--resident-nodes",
        "196608",
        "--collection-work",
        "100000000",
    ];
    let mut expected_proof = None;
    let mut expected_claim = None;
    for producer_ms in ["30000", "7200000", "14400000"] {
        let mut args = vec![
            "prove-artifact",
            "compiler",
            "--input",
            "job",
            "-o",
            "proof",
            "--force",
            "--time-ms",
            producer_ms,
        ];
        args.extend(compact);
        f.ok(&args);
        let proof = fs::read(f.0.join("proof")).unwrap();
        if let Some(expected) = &expected_proof {
            assert_eq!(&proof, expected);
        } else {
            expected_proof = Some(proof);
        }
        for verifier_ms in ["30000", "7200000", "14400000"] {
            let mut args = vec![
                "verify-artifact",
                "compiler",
                "--input",
                "job",
                "--proof",
                "proof",
                "-o",
                "compiled",
                "--emit",
                "program",
                "--force",
                "--time-ms",
                verifier_ms,
            ];
            args.extend(compact);
            let verified = f.ok(&args);
            let mut claim = verified["verification"].clone();
            claim.as_object_mut().unwrap().remove("elapsed_micros");
            assert_eq!(claim["physical_resource_claim"], "unattested");
            assert_eq!(claim["prover_observations"], Value::Null);
            if let Some(expected) = &expected_claim {
                assert_eq!(&claim, expected);
            } else {
                expected_claim = Some(claim);
            }
            assert_eq!(
                fs::read(f.0.join("compiled")).unwrap(),
                fs::read(f.0.join("generated")).unwrap()
            );
            f.clean();
        }
    }

    fs::write(f.0.join("destination"), b"previous").unwrap();
    for command in ["prove-artifact", "verify-artifact"] {
        for time_ms in ["0", "14400001", "18446744073709551615"] {
            let mut args = vec![
                command,
                "compiler",
                "--input",
                "job",
                "-o",
                "destination",
                "--force",
                "--time-ms",
                time_ms,
            ];
            args.extend(compact);
            if command == "verify-artifact" {
                args.extend(["--proof", "proof"]);
            }
            let rejected = f.run(&args);
            assert_eq!(rejected.status.code(), Some(1));
            assert!(rejected.stdout.is_empty());
            assert!(String::from_utf8_lossy(&rejected.stderr)
                .contains("limit time_ms must be in 1..=14400000"));
            assert_eq!(fs::read(f.0.join("destination")).unwrap(), b"previous");
            f.clean();
        }
    }
}
