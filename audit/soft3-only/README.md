# Joy belongs to the soft3 execution/proof stack

Owner-directed correction, measured 2026-09-27 on macOS arm64. Local development
validation; no release promotion, version bump or default-branch change.
Source revisions, commands, dependency/binary identities and summaries are in
[validation.json](validation.json). The installed smoke is preserved in
[installed.json](installed.json); the native compiler run in
[compiler-profile.json](compiler-profile.json).

## Cause and correction

Joy directly linked `trisha-rs` and its Triton prover. Zheng itself had no
foreign backend dependency: Joy selected its own private adapter and called
Trisha over a Zheng-derived relation. That adapter, private-state adapter,
verifier dispatch and all foreign Cargo patches/dependencies are removed.
There is no optional foreign-backend feature left in Joy.

Source commits:

- Joy `06aac01d2541c17f5b6295c04fd7eb2f4d7bb9e8`: native public proof paths,
  explicit private-proof refusal, dependency removal and boundary gate.
- Trident `7b1d4c069961fae099f40bb160328c09b8d15607`: default `external-targets`
  feature preserves the complete compiler; consumers can omit foreign target
  discovery, legacy import aliases, catalog and historical RAM/TASM resources.
- Trisha `aa25e32fd9005b88547b12348ac581750739ecf2`: explicitly retains that
  feature for its compiler, preserving the external warrior's behavior.

Both Joy compiler dependencies disable default features. Generic language/IR
code remains shared; this does not claim that every occurrence of a foreign
VM name disappears from compiler diagnostics or portable source comments.
Cargo features are additive, so the resolved package/feature closure is gated.

## Preserved and removed behavior

Preserved: source/bundle/raw execution on nox, native SDK imports, JOB1/RES1/ART1,
secret execution and explicitly labelled re-execution, public Zheng execution
certificates and authenticated public state certificates. Public protocols keep
their program/input/output/cost and state-root bindings.

Removed: private/ZK and hidden-query proving in Joy. `--zk` or nonempty proving
secrets fail before loading source/state or writing output. Legacy statement
provers also refuse secrets. Retired JOYZK envelopes fail explicitly, even with
legacy opt-in or a requested re-execution claim. Legacy verifiers require their
exact format. There is no public-certificate fallback for a secret witness.
Native zero-knowledge proving needs its own future soft3 implementation.

Historical audits and release notes retain their original evidence. Current
README, capabilities, CLI contract and compiler-facing references describe the
new boundary. The older Zheng replaceable-backend contract still describes the
independent Trisha checker; it does not establish current Joy capability.

## Evidence

All commands below use the source revisions above; full argv and outputs are
linked in the receipts. No Trisha directory exists beside the tested Joy root.
The separate compatibility root supplies Trisha with its pinned vendor tree.

- `python3 scripts/check-soft3-boundary.py`: 140 resolved packages, compiler
  features `[]`, no foreign package family. The lock removes 110 packages from
  the prior 250; every retained name/version/source tuple already existed.
- Joy `cargo check --workspace --all-targets --all-features --locked --offline`
  and `cargo test --workspace --release --locked --offline`: zero Rust warnings,
  124 passing tests, no ignored tests. Covers secret execution, all public/legacy
  prover refusal paths, historical headers, untouched output files, state and
  public proof tampering, typed input and control-flow regressions.
- Trident `cargo test --lib --locked --offline`, with defaults and separately
  `--no-default-features`: 724 passing tests in each configuration. Six named
  import/build/target integration suites pass 26 tests. `--test differential`
  passes 14 default / 13 minimal tests; the omitted test covers resources absent
  from the minimal compiler. Native differential coverage stays enabled.
- Trisha `cargo check --workspace --all-targets --locked --offline` and
  `cargo test --workspace --release --locked --offline`: zero Rust warnings,
  430 passing tests and six existing ignored cases. `trisha bench`: all 133
  fixture rows and 43 manual baselines pass; rows match the source-capacity run.
- `check-installed.py --joy .../install-committed/bin/joy --output ...`: 34
  commands with an isolated executable and system-only PATH. Supported public
  proving works, mismatched claims/budgets fail, secret execution works, private
  proving preserves output, foreign imports/targets fail, native SDK works.
- Trident `audit/self-hosting/check-generated-compiler-profile.py --joy
  .../install-committed/bin/joy --output ...`: all 65 commands / 21 observations
  pass. All 204 fixture files match the previous source-capacity acceptance,
  including the C1 program with SHA-256
  `4aed7fc83be96156fcb65c3bbb369c192ad27f894ab78a030e3588056a66d112`.
  This is the generated bounded compiler acceptance, not complete C2/C3 self-hosting.

The long full native compiler corpus was not rerun for this packaging change;
its language source and the measured C1 program are unchanged. No cross-platform
release validation or native dynamic proof capability is claimed.

## Binary measurement

`check-installed.py` records file sizes, hashes, `strip -x`, ad-hoc signing and
execution after stripping. Baseline is the earlier Hello World receipt named
in validation.json, with Joy `a15adb7` and Trident `9d52ee8`.

| macOS arm64 file | Before | After |
|---|---:|---:|
| Installed Joy | 8,389,808 bytes | 4,571,120 bytes |
| Stripped and ad-hoc signed copy | 7,175,792 bytes | 4,001,360 bytes |
| Hello World nox formula | 292 bytes | 292 bytes |

`nm --demangle --defined-only` finds no Trisha/Triton/tasm/twenty-first symbols.
All seven legacy compiler source files are absent as embedded resources; all
87 native nox compiler modules remain present. The binary still includes source
compilation, public proving, serialization and CLI functionality. Execution-only
packaging is a separate optimization and was not introduced here.
