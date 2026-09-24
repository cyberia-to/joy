# Exact source packages for SH2

Source revision: `99474821c893a6f0ff190d4f95f20ee4e3c7289a`.
This delivery adds `joy pack-job`: an explicit JSON manifest and exact source
files become a canonical `JOB1`, bound to the complete compiler artifact.
The contract is [source-packages](../../specs/source-packages.md).

Logical module paths and cfg flags are sorted; file paths never enter package
identity. Unknown fields, duplicate JSON keys, duplicate module paths, invalid
options and exceeded limits fail before publication. Source bytes retain CR/LF,
NUL and invalid UTF-8 for guest diagnostics. The host performs no language work.

[Validation](source-packages-validation.json) records exact revisions, commands
and log digests: 120 Joy tests pass, with zero Rust warnings; Trisha verifies
133 fixtures and 43 independent baselines with unchanged result/cycle rows
against the preceding compiler-profile delivery. Tests compare complete DAG
bytes with the independent reference builder, including non-power-of-two trees,
padding, aggregate source limits, path changes and manifest reordering.

[Installed CLI acceptance](source-packages-cli.json) records four commands:
pack the independent golden job, execute its guest, execute the extracted
artifact to obtain 14, and reject an exceeded source limit while preserving
the existing output. Reproduce from this repository:

```sh
python3 audit/self-hosting/run-source-packages.py \
  --joy ../install/bin/joy --output /tmp/source-packages-cli.json
```

The fixture exercises transport and returns a quoted program. SH2 still
requires a guest compiler that parses and compiles fresh source packages.
Regular file reads and JSON decoding precede the cooperative worker timer;
manifest storage uses the host byte cap. Full compiler-scale resource bounds
and native execution proofs remain later gates. This is local development
validation on the recorded host, without a release-candidate claim.
