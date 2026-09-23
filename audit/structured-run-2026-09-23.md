# Structured native execution — 2026-09-23

Joy now loads an exact raw-profile ART1 program plus an arbitrary complete
NOXDAG01 input, executes pure nox without retaining a trace, and atomically
publishes the complete result. Compiler JOB1 profiles remain rejected until
production binding/admission is implemented. [Contract](../specs/structured-run.md).

Observed CLI fixtures, with budgets/limits fixed to documented defaults:

| Fixture | Output | Charged reductions | Allocated nodes | Peak frames |
|---|---|---:|---:|---:|
| add14 | atom14 | 3 | 14 | 2 |
| identity_tree | exact input topology/particle | 1 | 13 | 1 |
| compact loop4097 | atom4097 | 61460 | 12320 | 8198 |

The loop's program is one reusable hand-authored formula, not source-level loop
lowering or compilation. Its observed worker elapsed time was133129 microseconds
on the pinned macOS aarch64 host. This is one execution observation, not a
portable benchmark. [Raw reports](structured-run-observations-2026-09-23.json)
include full program/input/output particles and actual timing values.

Reserved arena storage was26214416 bytes, frame buffer2883584 bytes and worker
stack268435456 bytes. These are reservations; RSS was not measured. Deadline
supervision is cooperative, as specified, and does not time-bound file reads or
preempt individual codec/allocator/filesystem operations.

The full workspace passed85 tests with zero ignored/failed, including existing
native/state/private proof regressions. Structured coverage adds10 library
cases, three CLI cases and one publication regression. Checks include shared
100-level DAG preservation, runtime-generated formulas, exact node/frame/budget
limits, input and output transport rejection, malformed ART1 arity/terminators/
profiles, forbidden services, actual worker deadline failure, FIFO/link refusal,
no-overwrite and preservation of existing output on failed execution/export.

Review found a staging/output-name collision in the old writer; the shared
writer now excludes the destination using case-neutral candidate names, and a
regression verifies both force modes and cleanup. Existing build publication
checks follow the new staging scheme. Focused tests after expanding loop fixtures
and capability text passed; all-target check and fixture freshness passed.
[Source/revision/log hashes and exact commands](structured-run-validation-2026-09-23.json).

Generate/check CLI fixtures with:
`CARGO_TARGET_DIR=../trident/target cargo run -p joy-rs --release --locked --example artifact_fixtures -- cli/tests/artifact_vectors.json --check`.

This delivery does not close SH1's source-data/control-flow or compiler-job gates.
It does not extend Zheng proof coverage. Main and release package versions are
unchanged; integration targets release/0.4.
