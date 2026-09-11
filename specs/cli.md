---
tags: joy, cyber, specs, cli, warrior
crystal-type: spec
crystal-domain: joy
status: draft
alias: joy cli contract
---

# joy CLI

Owner: joy. The baseline describes source version 0.4.0 at `6b8dfd3`.
The working-tree integration below records concurrent target-package work
separately from that committed baseline. Target
sections define requirements for a subsequent interface revision and remain
unimplemented until conformance tests pass. MUST/SHOULD/MAY are normative
within those target sections.

Cross-repository contracts:

- [soft3/specs/execution-model.md](../../soft3/specs/execution-model.md): foundational VM/OS, warrior, backend, worker and open network-instance model;
- [cyber/specs/cli.md](../../cyber/specs/cli.md): node lifecycle and worker management;
- [cyber/specs/worker.md](../../cyber/specs/worker.md): authoritative job and acceptance contract;
- [zheng/specs/execution.md](../../zheng/specs/execution.md): execution certificate semantics.

## responsibility

Joy is a warrior implementation; workers instantiate its capabilities through
selected backends. The same supported VM/OS/proof combination MUST accept an
open-ended set of compatible network-instance descriptors. Bundled presets
are conveniences; adding a compatible instance must require configuration
only. CPU/GPU and local/remote placement are executor choices. This is the
target architecture; today's stateless cyber alias has no network binding.

Joy's target lifecycle is build → run/prove → verify → deploy. Cyber owns
network state, job scheduling and acceptance. Trident owns the compiler
contract and ProgramBundle; Joy's build command uses its reference nox
lowering. The implemented CLI currently exposes run/prove/verify/describe;
build and deploy are specified below as target commands. `--target cyber`
selects a supported target alias; it supplies no network connection or
authority to modify chain state.

## implemented command grammar

```text
joy describe [--target nox|cyber]
joy run INPUT [COMMON]
joy prove INPUT [COMMON] [--output PATH]
joy verify ARTIFACT [COMMON] [--claim VALUES]
joy verify INPUT --proof ARTIFACT [COMMON] [--claim VALUES]
joy verify INPUT --claim VALUES [COMMON]
joy verify ARTIFACT --legacy-trace-statement
joy --help
joy --version
```

INPUT accepts a `.json` ProgramBundle, `.tri` source, project directory or
raw `.nox` formula. Raw formulas are wrapped in a nox bundle. Proof artifacts
are identified by their content format; the normal output suffix is `.zheng`.
Paths resolve relative to the caller's working directory.

COMMON flags currently appear on run/prove/verify:

| flag | default | current meaning |
|---|---|---|
| --target nox\|cyber | nox | supported target alias; other targets fail |
| --profile NAME | debug | passed to compilation for source inputs |
| --input-values V1,V2 | empty | ordered public u64 values |
| --secret V1,V2 | empty | native execution witness values; proof mode refuses secrets |
| --budget N | 1000000 | reduction limit; proof verification also checks the certificate budget |
| --state VALUE | absent | rejected explicitly; state loading is unavailable |

`--profile` is described by help as debug/release, but the parser accepts a
string; downstream compilation determines validity. Public inputs become
the subject `[p_last [... [p_first 0]]]`. One native trace row consumes one
reduction unit. Budget is not a wall-time or memory limit.

## current output and exits

| operation | stdout | stderr |
|---|---|---|
| describe | versioned Trident target package JSON | errors |
| run | one decimal output value per line | reductions and errors |
| prove | saved artifact path | timing, reductions, bytes and public output |
| verify execution certificate | PASS and checked public coordinates | failure diagnostics |
| verify by rerun | labelled PASS/FAIL; mismatch details | runtime errors |
| legacy inspection | labelled statement result and unverified metadata | refusal/errors |

Success exits 0. Application failures, proof rejection and claim mismatch
exit 1. Clap syntax errors exit 2. Help/version exit 0. Verification output
is human-readable; consumers MUST NOT infer a stable machine schema from it.
Current proof save behavior may overwrite an existing path; target publication
rules below intentionally tighten this behavior.

## three different verification meanings

1. `run` produces native execution output and measured reductions.
2. `verify INPUT --claim ...` repeats native execution and compares outputs.
3. `verify ARTIFACT` or `verify INPUT --proof ARTIFACT` checks a public
   execution certificate without running nox.

Current execution certificates use `zheng-nox-public-execution-v1`, disclose
the full witness and have linear verification cost. They authenticate the
supported program, public input/output and reduction coordinates. With
`--proof`, Joy additionally compares the certificate's program with INPUT.
With `--claim` or `--input-values`, Joy compares the requested vectors.
Without those flags, verification checks the artifact's own statement;
the caller must still establish that this is the intended computation.

The public proof profile refuses secret inputs, state reads and unsupported
symbolic operations. Complete nox coverage, authenticated state execution,
zero knowledge and deployment remain unfinished.

Legacy `zheng-hypernova-tensor-merkle-v2` artifacts require
`--legacy-trace-statement`. This checks a legacy statement and reports
execution/output as unverified. Requested input/output/state/secret
constraints are refused. Legacy inspection cannot satisfy a cyber worker
execution-certificate request. Proof verification MUST never downgrade to
rerun or legacy inspection after rejection.

## baseline gaps requiring correction

All CLI `--state` requests now fail before execution. The stateless library
Runner also rejects bundles marked `reads_state`. These guards do not provide
state loading or authenticated state execution.

`describe` reports the versioned Trident target package and separate native
execution/public proof restrictions. It is compiler/warrior discovery;
it does not implement the planned worker protocol capability envelope.

`--secret` currently exposes native witness arguments through the process
command line. It remains a development compatibility option; the worker
adapter must use explicit private channels. CLI cancellation, hard memory
limits, machine output and atomic no-overwrite publication are target work.

## target additions

Concurrent working-tree integration introduces `joy describe --target
nox|cyber`, emitting the versioned Trident target package as JSON. Runtime
declarations live in `joy/targets/nox/capabilities.json`; machine constants
come from Trident's upstream nox contract. It also changes all CLI `--state`
requests to explicit errors. These changes are separate from this spec-only
commit and require their own implementation validation.

Use `describe` as the discovery command. Extend its versioned runtime
declarations for the worker contract rather than adding another discovery
command. Preserve the existing target-package schema and compiler consumers;
incompatible package changes require its own schema/API version transition.

Preserve the existing command names and add:

```text
joy describe --target nox
joy build INPUT [--target nox|cyber] [--profile NAME] [--emit bundle|nox] [-o PATH] [--force] [--format json-v1]
joy run INPUT --input-file public.json --format json-v1
joy run INPUT --input-file public.json --secret-file private.json
joy prove INPUT --input-file public.json --output proof.zheng --format json-v1
joy verify INPUT --proof proof.zheng --input-file public.json --claim-file output.json --format json-v1
joy deploy BUNDLE --target cyber --state INSTANCE --dry-run [-o PLAN] [--proof ARTIFACT] [--force] [--format json-v1]
joy deploy PLAN --submit --rpc URL --signer REF [--format json-v1]
```

## target build contract

`joy build` provides the standalone compilation entry point, requiring no
node, wallet or live network. It accepts a `.tri` source or project directory.
Default target is nox and default compilation profile is debug, matching
run/prove. Explicit target selection overrides the project; otherwise use
the project target and then nox. Named profiles use Trident's project
resolution rules. Unknown profiles and unsupported targets fail explicitly.

Build selects the version-matched target package and invokes Trident's
compiler library. It MUST use the same resolution and lowering path as
Joy's source run/prove commands. It MUST NOT spawn `trident build` and risk
a delegation loop. The nox reference lowering remains owned by Trident.

Default `--emit bundle` writes a ProgramBundle JSON file named
`<source-stem>.bundle.json` beside a source, or `<project-name>.bundle.json`
in the project root. It preserves assembly, entry point, function metadata,
source identity, target and state-read declarations. `--emit nox` writes
assembly to the corresponding `.nox` path; `-o`/`--output` overrides either
destination. Assembly-only export is explicitly a lower-metadata format;
deployment requires the full bundle.

Library builds retain callable definitions; run/prove require an executable
entry. Building a state-reading program may succeed when compilation supports
it, while run/prove must still refuse unavailable state capabilities. Build
success establishes compilation, independently of runtime/proof coverage.

Build MUST NOT execute the program, request witnesses, generate a proof or
submit anything to a node. Identical source/dependency bytes, compiler and
target-package versions, and profile must yield identical artifact bytes;
timestamps, absolute checkout paths and measured wall time stay outside the
artifact identity. Static costs must be labelled estimates; absent costs
must not be reported as measured zero cost.

Human-mode stdout contains the output path; diagnostics go to stderr. JSON
results carry kind `build`, artifact path/format, source and program identity,
compiler version, target-package identity and profile. Publication follows
the atomic no-overwrite/explicit-force rules below.

Current gap: `joy/cli/main.rs` has no Build variant. Source compilation already
exists in `joy/cli/compile.rs`. `trident build --target nox` currently uses its
own reference path; adding `joy build` does not require replacing that path.
Bundle equivalence between the two library entry paths is a conformance gate.

## target deploy contract

Deployment publishes an immutable program bundle under a network's rules.
It returns evidence of submission or acceptance. Program execution and
activation of stateful behavior are separate operations requiring that
network's explicit policy. Joy constructs and transports deployment requests;
Cyber owns admission, storage and finality.

The command has two explicit modes: `--dry-run` prepares a deployment plan;
`--submit` submits a previously prepared plan. Exactly one mode is required.
A bare `joy deploy INPUT` is a usage error. Neither mode silently builds or
proves: use build/prove first, then freeze those artifacts.

Preparation consumes a full ProgramBundle, the requested network/state
descriptor and any proof required by its deployment policy. It checks target
compatibility and creates a plan binding the exact artifact bytes/identities,
network/genesis identity, network ABI/policy version, operation and attached
proof. The plan contains the submission payload; source-file paths alone
are insufficient. Commitments and encoding follow the network's versioned
deployment format, which must be defined before preparation is implemented.

Dry run is offline: it does not sign, broadcast, spend funds, execute user
code or assert live admission. Without `-o`, it prints the plan; with `-o`,
it writes the plan atomically and prints its path. Missing network descriptors
or an unsupported deployment format fail, including during dry run. A bare
nox target has no deployment destination; the current stateless `cyber`
alias also lacks the required network deployment descriptor.

Submission requires an explicit RPC endpoint and signer reference. The signer
reference identifies an authorized signing service/key handle; secret key
material MUST NOT appear in argv or plan files. Joy checks the endpoint's
network/genesis identity, validates the frozen plan and any attached proofs,
and obtains authorization for that exact payload. Signer refusal stops the
operation. An unsigned chaosnet `/v1/link` call cannot fulfill this contract.

The node independently validates authorization and deployment policy.
Submission success MUST report `submitted` and the actual receipt identifier;
`accepted`/`finalized` may be reported only with corresponding node evidence.
Local content hashing or HTTP reachability supplies neither receipt nor
finality. A lost reply after sending yields `submission_unknown` with a
stable submission identity for reconciliation, never blind retry with a new
identity. Repeated submission of the same authorized operation must follow
the network's idempotency/replay rules.

JSON results have kind `deployment_plan` for preparation or `deployment` for
submission and identify the bound program, network/genesis, policy and status.
Receipt/finality fields are null until evidence exists. Plan output through
`--format json-v1` uses the common result envelope. Verification/admission
rejection uses exit 5; unsupported deployment uses exit 3; ambiguous transport
failure uses exit 1 and error code `submission_unknown`, `retryable=false`.

Deployment and proof capabilities are independent. Publishing public program
code need not require an execution proof unless network policy says so.
Stateful activation or proof-gated deployment must require the appropriate
authenticated statement; stateless proof support cannot imply it.

Current gap: Joy has no Deploy CLI variant and its Deployer trait implementation
returns an error. Runtime metadata correctly declares `deploy=false`.
Dry-run preparation alone MUST NOT change that flag to true. The current
Trident deploy command publishes through its registry path, separately from
warrior network deployment. Aligning delegation and the richer plan/signer/
receipt API requires a Trident wiring change; the existing Deployer trait
alone cannot express this contract.

## target inputs, publication and discovery

`--input-file`, `--secret-file` and `--claim-file` accept a UTF-8 JSON array
of canonical decimal strings. They are mutually exclusive with their
respective inline value flags. `-` means stdin; at most one input may claim
stdin. Unknown format, noncanonical field values, oversized inputs and
conflicting flags MUST fail before execution. Values outside the pinned
field range MUST be rejected rather than silently reduced.

Secrets MUST be absent from argv, logs and diagnostics in automated use.
Public proof requests with secret data MUST fail before an artifact is
produced. File/pipe handling must respect the caller's access controls;
memory cleanup is best effort unless a stronger guarantee is implemented.

`--output` publication MUST default to no overwrite, write a temporary file,
flush it and atomically publish a complete artifact. A new explicit `--force`
may replace an existing artifact only after successful production. Failure
must leave the old artifact intact and remove temporary output.

`describe` runtime declarations MUST report backend version, supported VM/proof/worker contract
versions, operations, proof disclosure/succinctness, state/secret support and
enforceable limits. Unsupported capabilities remain false. Joy currently has
no worker daemon command; a Rust adapter is the first integration path.

## target machine results

`--format json-v1` is opt-in. Exactly one JSON object plus newline appears on
stdout; progress stays on stderr. Existing default outputs remain compatible.
An execution example is:

```json
{"schema":"joy/cli/v1","command":"run","ok":true,"result":{"kind":"execution","output":["8"],"reductions":"7"},"error":null}
```

These are illustrative values, not a benchmark or fixture for a particular
program. All u64 values use decimal strings; flags use JSON booleans.
Application failure sets `ok=false`, `result=null`, and an error object
with stable `code`, safe `message` and boolean `retryable`. Syntax errors
before machine-mode selection may use stderr only.

Prove results MUST carry kind `execution_certificate`, proof format,
artifact path, byte size, program identity, authenticated public input/output,
reductions and disclosure mode. Identity encodings follow the proof profile.
Verification results MUST identify mode `execution_certificate`,
`reexecution` or `legacy_statement` and list which external expectations
were checked. Missing expected output means `claim_checked=false`, even
when the artifact's own output is authenticated. Legacy output must remain
explicitly unverified. Wall-time telemetry is distinct from proven cost.

Target application exits match the cyber finite-command convention:
0 success; 1 operation failure; 2 usage/input validation; 3 unsupported
capability; 4 resource unavailable/busy; 5 proof or claim rejection;
130 handled cancellation. OS termination retains platform status.
Baseline code 1 remains valid until the new interface ships.

A library adapter returns typed results and errors directly. It MUST NOT
invoke a shell or scrape these human-readable CLI strings. Cyber's worker
contract owns job IDs, retries, deadlines and acceptance independently of
the standalone CLI envelope.

## conformance and rollout

Implement in this order:

1. Validate the concurrent state-refusal and target-description changes;
   reject incompatible verification modes.
2. Add build using the shared compiler path and deterministic bundle output.
3. Extend describe capabilities; add file inputs and structured output/errors.
4. Add atomic artifact publication and adapter-enforceable limits/cancellation.
5. Wire public stateless jobs through the cyber worker adapter.
6. Define the Cyber deployment format/policy and signer/receipt contract;
   implement dry-run preparation, then authorized submission.
7. Add stateful/secret profiles only with authenticated proof support.

Required cases: native output and rerun agreement; proof verification without
rerun; wrong program/input/output/budget rejection; corrupt proof rejection
without downgrade; state/secret refusal; exact JSON types and exit behavior;
artifact preservation on failure; cancellation and oversize input rejection.
Public execution and legacy state-proof suites must be reported separately.

Build gates: standalone build without a node; artifact equivalence across
checkout locations; target/profile precedence; imported library preservation;
built-bundle run/prove/verify equivalence with source inputs; explicit failure
without replacing existing output. Deploy gates: dry run has no network or
signer effects; missing network support fails; modified plan/bundle/proof and
wrong genesis fail; signer refusal prevents submission; duplicate delivery
and lost responses never fabricate acceptance or duplicate state effects.

Baseline sources: `joy/cli/{main,run,prove,verify,execution_verify}.rs`,
`joy/rs/{warrior,execution}.rs`. Current tests include
`joy/cli/tests/execution_claim.rs`, `joy/cli/tests/target_package.rs` and
`joy/rs/tests/public_execution.rs`.
The three legacy state-proof acceptance failures documented in README remain
open; this specification does not convert them into passing coverage.

Breaking types/semantics require a new schema or proof profile. Additive
result fields are allowed; consumers ignore unknown result fields, while
request/config parsers reject unknown fields. Keep help, this specification,
Trident delegation expectations and conformance tests aligned. Requests for
Trident changes go through `joy/.claude/trident-wiring.md`.
