# Joy published release validation — 2026-09-16

Joy 0.5.0 is published with Trident 0.3.0 and Trisha 0.3.0. The [current report](release-2026-09-16.md) records the exact released artifacts and all completed acceptance gates.

## Historical validation — 2026-09-12

The following checkpoint predates the release and preserves its original scope and evidence.


Status: implementation and artifact preparation. No new release is published.
The active cross-repository gate ledger is
[Trident's release record](../../trident/audit/full-release-preparation.md).

The complete Joy workspace passed **60 tests**, with zero failures or compiler
warnings, using Trident compiler API 2 and pinned Triton 7. The suite includes
source/build identity, fresh public execution verification, real private
execution/state STARKs, wrong program/input/output/root rejection and explicit
legacy-proof boundaries. Receipt: `/tmp/joy-api2-triton7-release.log`.

The downstream proof/commitment dependencies separately pass their complete
release suites: Zheng 190 tests, Lens 129 all-feature tests and BBG 71 tests.
Current nox admission passes 169 default / 175 all-feature tests. These counts
describe separate test inventories, not additional Joy tests. Exact source
fixes and proof arguments are in the owning repositories' audits.

The verifier binds the actual computation to its public result. For private
execution it independently regenerates the exact Zheng CCS checker and full
public statement before verifying Trisha's randomized native STARK. Public
state binds each selected namespace, key, value and all four root coordinates;
private queries select cells inside the proof from authenticated public tables.

Remaining acceptance includes fresh source/archive installs and smoke checks
against the exact final binaries on macOS and Linux. Dynamic continuations and
variable shapes are still outside the bounded execution relation. The private
query path requires all ten public dimension tables and does not implement a
hidden database. Node/state synchronization, live deployment and independent
cryptographic assurance remain separate open full-release requirements. The
current owner has not accepted a narrower release scope.

Protocol migration:

| Artifact | Current identifier | Required action |
|---|---|---|
| Public execution | `JOYEXEC2` | Regenerate legacy trace statements as execution proofs |
| Private execution/query | `JOYZK003` | Regenerate Triton 2 / JOYZK001 / JOYZK002 artifacts |
| Authenticated public state | `JOYST001` | Regenerate against commitment-v2 roots and opening-v3 certificates |
| Warrior metadata | compiler API 2, package schema 1 | Install coordinated Trident/warriors |

## Current post-loop-return and BBG closure validation

`RAYON_NUM_THREADS=4 RUST_TEST_THREADS=4 cargo test --release --workspace --locked`
passed **62 tests**, zero failures, ignored tests or Rust compiler warnings,
across11 test/doc-test binaries, exit0. Receipt:
`/tmp/joy-post-nox-return-full.log`.

This is the current working tree after Trident's bounded nox loop-return and
aggregate lexical-scope fixes and BBG's external neuron-id dependency addition.
The Joy lockfile received only the diagnosed local neuron-id0.1 package/BBG
requirement delta; no registry dependency versions were updated. The run
includes fresh-process public/private/state verification and both new loop
proof regressions: six public certificates across both source profiles and a
real Triton7 private proof that skips a later secret read after return. Changed
public input/result/cost reject. This supersedes the earlier60-test Joy receipt;
separate platform/archive and downstream dependency evidence is not inferred
from these62 tests.
