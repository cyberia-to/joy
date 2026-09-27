# Native private proving and Joy operations

This is development-branch evidence for `release/0.4`, not a promoted release.
Joy source: `f09800ba7146bd94b712af7060346e90cc9b79b6` (native proof delivery
`015a127`, native operations `77c7bb8`, worker verifier `f09800b`). Zheng proof
source: `6b5a8a85535efb8d2852736af738278a9bd92db7`; CLI wording correction:
`50bbacd9837896e030b0c6649387d651de23c529`. Exact sibling pins, commands,
toolchains and log digests are in [validation.json](validation.json) and
[installed.json](installed.json). The source review is [review.md](review.md).
Documentation changed after the measured source revision; these receipts name
the implementation they actually exercised.

## Delivered behavior

Joy executes on nox and proves through Zheng's native Goldilocks/Hemera backend.
Secret witnesses and explicit `--zk` select `JOYZH001`. The verifier checks the
canonical program, public input/output, reduction count, budget and build/state
identity without a private witness or native rerun. Authenticated state proofs
hide query coordinates over complete public BBG tables. Explicit public proof
APIs retain full witness disclosure and reject secrets.

Successful native runs consume exactly the supplied active witness stream.
A bounded versioned input file keeps witness values out of child argv. Proof
publication is atomic and requires explicit replacement. Batch operations run
isolated jobs, `test` executes compiled assertions, and `bench` measures the
specified computation with independent optional expected output.

The library worker verifier checks all four public/private and stateless/state
profiles against a saved job. Its result carries verified computation values.
Cyber retains network scheduling, root policy, persistence and acceptance;
this delivery adds no network endpoint, reward authorization or finality receipt.
Trisha's Neptune transactions remain owned by Trisha.

## Validation

From the source revision above:

```sh
CARGO_TARGET_DIR=../target cargo check --workspace --all-targets --locked --offline
CARGO_TARGET_DIR=../target cargo test --workspace --release --locked --offline
python3 scripts/check-soft3-boundary.py
CARGO_TARGET_DIR=../target cargo install --path cli --root ../install-committed --locked --offline --force
PYTHONDONTWRITEBYTECODE=1 python3 audit/native-private/check-installed.py --joy ../install-committed/bin/joy --output /tmp/joy-native-private-installed.json
```

The release suite passed151tests with zero warnings. The installed acceptance
passed51commands in an isolated directory with only system utilities on PATH,
including native private proofs, separate-process verification, wrong-result
rejection, witness stream/publication failures, batch proofs, tests and bench.
No Trisha sibling was present. The all-features boundary resolved145packages and
no external Trident target feature. Its manifest graph and retained symbols
contain no foreign proving backend; all87native compiler SDK resources remain.

Additional `cargo check --workspace --all-targets --target TARGET --locked
--offline` gates passed without warnings for macOS x64, Linux arm64/x64 and
Windows x64, using the Rustup toolchain recorded in `validation.json`.
These are compilation checks. Executable/runtime acceptance was on macOS arm64.
The first cross-check attempted with Homebrew Rust lacked target stdlibs; the
successful gates explicitly select the installed Rustup target toolchains.

## Measurements

Commands, input sources, binary hashes and timings below are recorded in
`installed.json`, from the source and sibling revisions above. This is one local
sample with other validation jobs running, not a stable performance benchmark.
The private fixture returns the product of two `divine()` inputs supplied by file.

| Observation | Measured value |
|---|---:|
| Installed Joy binary | 4,910,896 bytes |
| `strip -x` plus ad-hoc signing | 4,292,112 bytes |
| Compiled Hello World nox formula | 292 bytes |
| Private product artifact, public result91 | 292,063 bytes |
| Prove process elapsed | 234,997,417 ns |
| Verify process elapsed | 114,284,042 ns |
| Authenticated native reductions | 32 |

The new binary includes the private backend and added CLI operations together;
its difference from earlier Joy is not an isolated prover-only size measurement.
Native proof entropy randomizes bytes and may change proof length across runs.

## Compiler regression gate

The same installed binary passed the generated compiler profile:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 ../trident/audit/self-hosting/check-generated-compiler-profile.py --joy ../install-committed/bin/joy --output /tmp/joy-native-private-profile.json
```

[compiler-profile.json](compiler-profile.json) records65commands and21observations
against Trident90f42b4 and the Joy source/binary above. All204retained fixture
files match `audit/soft3-only/compiler-profile.json` by path, length, SHA-256 and
DAG metadata. C1 is9,191,495bytes; SHA-256
`4aed7fc83be96156fcb65c3bbb369c192ad27f894ab78a030e3588056a66d112`.
This establishes regression-free bounded compiler operation. The complete
compiler's C2/C3 self-compilation milestone remains open.

## Remaining boundaries

The protocol is arithmetic MPC-in-the-head with219repetitions, fresh OS entropy,
canonical bounded wire data and exact residual checks. Its security assumptions
are specified by Zheng in `specs/native-private-ccs.md`. Source review and tests
supply implementation evidence; an independent production cryptographic audit
has not been performed. Proof generation, storage and verification have linear
costs, bounded by the admitted circuit and256MiB artifact cap.

Program, public IO, selected reductions and authenticated state tables remain
public. Arbitrary dynamic continuations, differently shaped branches and complete
compiler self-proofs remain outside the existing static relation. The current
worker boundary does not bind a network nonce/height into the proof or implement
network job admission. Private database synchronization is also separate work.
