# Explicit compacting compiler deadline

Joy permits an explicitly requested `--time-ms` through 7,200,000 ms when
bounded compaction is selected. The production change is one hard ceiling in
`rs/structured/limits.rs`; `specs/structured-run.md` owns the contract.
The default stays 30,000 ms and ordinary execution stays capped at 300,000 ms.
Reduction, allocation, resident storage, frame, collection-work, transport and
JOB1 limits are unchanged. No automatic retry or deadline escalation is added.

The measured trigger is Trident CI run `36353842247`, head
`23691cd2c6885bf25bfc023799552559724dbc2b`, Intel macOS job `108717527925`.
Its first full C1(S1)→C2 run returned `Execution(Cancelled)` at the selected
3,600,000 ms deadline. The command took 3,600,087,115,845 ns and reported
8,602,998,808 evaluator checkpoints, 161,210,702 cumulative allocations and
55 collections. Charged reductions on that failure are unknown.

`ci/observation.json` records the exact invocation and these counters;
`ci/c2-step.json.gz` preserves the producer receipt and its raw command output.
The original 8,537,516-byte GitHub artifact, API metadata, platform receipt and
raw job log are retained. `ci/archive-files.json` verifies all 356 original
archive members; `ci/files.json` binds every retained file and raw encoding.
The GitHub action deprecation notices remain in the raw log and are distinct
from Rust compiler warnings.

The failed compiler, JOB1 and inventory hashes exactly match the successful
local reference in `ci/local-same-input-success.json.gz`. That establishes a
measured machine-throughput limitation at the existing deadline. The new
7,200,000 ms ceiling supplies additional explicitly bounded elapsed time;
it is not a measured completion time, remaining-time projection or guarantee
that this Intel runner will finish. The original CI run remains failed.

Validation used Joy base `2878f4b17dfedf237c6110d7d411bb4824e65103` plus the
exact six source/spec hashes in `validation.json`, on branch
`feat/0.4-compiler-deadline`. The recorded local dependency state includes
unrelated dirty sibling files; these checks are local development evidence.
No default branch or Trident source was changed by this unit.

The final commands from the Joy repository were:

```sh
CARGO_TARGET_DIR=../target-joy-deadline cargo test --workspace --release --locked --offline
CARGO_TARGET_DIR=../target-joy-deadline cargo check --workspace --all-targets --locked --offline
python3 scripts/check-soft3-boundary.py
rustfmt --check --edition 2021 rs/structured/limits.rs rs/structured/tests/compaction.rs rs/structured/tests/compiler/compaction.rs rs/structured/tests/admission.rs cli/tests/structured_run.rs
```

All 172 tests in 27 suites passed; no failures, ignored tests or Rust warnings.
The all-target check, formatting check and soft3 boundary check passed. Exact
raw logs are compressed without normalization, with raw/stored hashes in
`validation.json`. Cargo metadata and toolchain versions are retained too.

The tests accept the exact new ceiling, reject one above it, preserve ordinary
and default deadlines, and exercise both the old and new ceilings. A raw loop
that collects repeatedly retains exact output, particles, charged reductions,
allocations, frames and all collection counters. A CompilerJob fixture also
retains exact RES1/extracted-program bytes and its entire report except elapsed
time. A short pre-aged worker start deterministically reaches actual evaluator
cancellation; the worker deadline test covers both append-only and compacting
no-output failure. CLI rejection at the new ceiling plus one preserves an
existing destination and leaves no staging file.
