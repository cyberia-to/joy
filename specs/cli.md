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

- [cyber/specs/cli.md](../../cyber/specs/cli.md): node lifecycle and worker management;
- [cyber/specs/worker.md](../../cyber/specs/worker.md): authoritative job and acceptance contract;
- [zheng/specs/execution.md](../../zheng/specs/execution.md): execution certificate semantics.

## responsibility

Joy executes, proves and verifies programs on nox. Cyber owns network state,
job scheduling and acceptance. Trident owns compilation and ProgramBundle;
Joy may invoke its compiler for local developer inputs. `--target cyber`
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
joy run INPUT --input-file public.json --format json-v1
joy run INPUT --input-file public.json --secret-file private.json
joy prove INPUT --input-file public.json --output proof.zheng --format json-v1
joy verify INPUT --proof proof.zheng --input-file public.json --claim-file output.json --format json-v1
```

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
2. Extend describe capabilities; add file inputs and structured output/errors.
3. Add atomic artifact publication and adapter-enforceable limits/cancellation.
4. Wire public stateless jobs through the cyber worker adapter.
5. Add stateful/secret profiles only with authenticated proof support.

Required cases: native output and rerun agreement; proof verification without
rerun; wrong program/input/output/budget rejection; corrupt proof rejection
without downgrade; state/secret refusal; exact JSON types and exit behavior;
artifact preservation on failure; cancellation and oversize input rejection.
Public execution and legacy state-proof suites must be reported separately.

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
