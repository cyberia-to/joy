# Production compiler transport acceptance

Date: 2026-09-24. Joy source `e67383e8f046015e985ac878dc37611b5f5da5af`;
Trident `23d3fdf842eb0024770194eb7469624a94d13ebd`;
Trisha `cf89bf3cee97581ba6bd4bae87d42de793045085`;
nox `5271961a72a1f1e9922df36e3e4e3c65d1acff81`.

Full dependency revisions, host identity, commands, log digests and original
fixture-export failure: [validation receipt](compiler-jobs-validation.json).
This is local development evidence on macOS arm64 / Apple M4 Max, separate
from CI and release candidates. No default branch was changed.

## Delivered behavior

Joy admits exact compiler-profile ART1 and JOB1, checks the full compiler
identity, validates canonical collections and package structure, and applies
requested limits beneath independent host ceilings. It preserves exact source
bytes without parsing them. Repeated shared subtrees consume logical visits.
Already loaded compiler/input nodes count against the tightened arena allowance.

The guest constructs RES1 on nox. Joy checks the complete job binding, success
profiles or ordered diagnostics before encoding/publication. The library returns
RES1 and optional extracted ART1 bytes; CLI `--emit result|program` atomically
publishes one selected file. Valid compile diagnostics, runtime failure and
successful compilation remain distinct outcomes.

## Gates at the revisions above

| Command | Result |
|---|---|
| Joy `CARGO_TARGET_DIR=../trident/target cargo check --workspace --all-targets --locked` | Pass, no Rust warnings |
| Joy `CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked` | 110 passed, no failures or ignored tests |
| Trisha `CARGO_TARGET_DIR=../trident/target cargo run --release --locked -p trisha -- bench` | 133/133 fixtures and 43/43 independent baselines verified |

Trisha result/cycle rows equal the previous collection delivery at Trident
`fb7d6962f34c2a7206d5403025928f39eeb004dc`.
No Trident library source changed here; prior formal collection audits remain
UNKNOWN and establish no library proof.

Tests mutate every compiler/result digest limb; reject malformed records,
padding, labels/order/options, missing entries and bad diagnostic spans/codes;
and cover arbitrary source bytes, duplicate diagnostics and valid EOF spans.
They enforce exact requested reduction/frame/arena/validation limits,
compiler-dominant input transport bounds and dynamically constructed oversized
RES1 containers. CLI failures preserve the old destination with `--force`.

## Installed CLI observations

Reproduce with
`python3 audit/self-hosting/run-compiler-jobs.py --joy ../install/bin/joy --output audit/self-hosting/compiler-jobs-cli.json`.
The [installed CLI receipt](compiler-jobs-cli.json) records the exact commands,
binary digest, identities and independent expected-byte comparisons. The runner
was repeated after rebuilding the committed Joy source above.

| Guest workload | Charged reductions | Allocated nodes | Peak frames | Input / output validation visits |
|---|---:|---:|---:|---:|
| Construct bound successful RES1 with quoted ART1 | 9 | 125 | 5 | 183 / 36 |
| Construct bound compile-error RES1 with duplicate diagnostics | 9 | 143 | 5 | 183 / 104 |
| Separately run the extracted ART1, returning14 | 1 | 10 | 1 | Raw profile |

The transport fixtures perform no source compilation. They exercise real guest
result construction and separately executable output. SH2 requires a source
compiler inside nox. Compiler-profile seed export and a native source/package
driver are the next integration work. Reservations and cooperative deadlines
are not process RSS/hard elapsed-time guarantees; SH4 retains compiler-scale
acceptance. Dynamic native proofs remain SH7/SH8.
