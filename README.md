# Joy

Interface specification: [Joy CLI](specs/cli.md). Node integration and
acceptance are owned by [cyber's worker contract](../cyber/specs/worker.md).
Both specs distinguish the implemented baseline from the planned interface.

Joy compiles and executes Trident programs on nox. Supported public programs
can also produce a Zheng execution certificate whose input, output, program
and reduction count are checked by the verifier.

```sh
joy describe --target nox  # versioned JSON package and runtime capabilities
joy run program.tri --input-values 3,5
joy prove program.tri --input-values 3,5 --output program.zheng
joy verify program.zheng --claim 24 --input-values 3,5
joy verify program.tri --proof program.zheng --claim 24
joy verify program.tri --claim 24 --input-values 3,5  # native re-execution
```

Public proving uses `zheng-nox-public-execution-v2`: Zheng derives the global
CCS from the canonical program, authenticates the complete public witness and
checks all constraints and public coordinates. This certificate discloses the
witness and has linear verification cost. Verification does not run nox.

Joy uses the soft3 execution/proof stack exclusively. `run --secret` supplies
execution witnesses. `prove --secret` and `prove --zk` are refused before
compilation or publication because the current soft3 execution certificate is
public, not zero knowledge. Historical private envelopes are unsupported.

`--state state.json` loads a bounded authenticated public BBG certificate.
Public state proofs use `joy-nox-public-state-execution-v1`; each actual lookup
binds namespace, index, returned value and all four root limbs to execution.
Hidden-query and zero-knowledge state proving are unavailable.

```sh
joy prove state.tri --state state.json --input-values 11 --output state.zheng
joy verify state.zheng --state state.json --claim 82
```

Dynamic continuations and variable branch shapes remain unsupported. Budgets
must cover the authenticated cost of the selected execution path. The relation
also bounds every possible cost below the field modulus. See
[Zheng's exact contract](../zheng/specs/execution.md) and
[proof backend/state contract](../zheng/specs/ccs-execution-backends.md).

Public inputs bind as `[p_last [... [p_first 0]]]`. Proof verification checks
requested `--claim` and `--input-values`; `--budget` bounds the certificate's
budget. Self-contained artifacts carry the program; passing a source/bundle
with `--proof` also checks that the certificate belongs to that program.

## Target ownership

`joy describe --target nox` exports the installed versioned Trident target
package as JSON without executing user code. `cyber` currently names the same
nox adapter; it does not advertise a network SDK or deployment.
Machine constants come from the canonical nox contract through Trident;
Joy owns [runtime capabilities](targets/nox/capabilities.json), including the
execution and proof restrictions. No checkout or ambient machine descriptor
is needed. State files are authenticated certificates, not live network sync.

## Legacy trace statements

Old `zheng-hypernova-tensor-merkle-v2` artifacts do not establish execution
or emitted values. Inspection requires an explicit opt-in:

```sh
joy verify old.zheng --legacy-trace-statement
```

This mode labels execution/output unverified and refuses requested IO, state
or secret constraints. There is no automatic proof downgrade. The old
`prove_zheng`/`verify_zheng`/`verify_artifact` library methods remain legacy
statement APIs; the `Prover`/`Verifier` traits use execution certificates.

## Build and status

Use coordinated sibling checkouts of Trident, Zheng, Lens, BBG, nox,
Hemera and strata. Joy selects Trident's no-default-features compiler surface:
portable libraries and the native nox SDK. External target discovery, legacy
foreign import aliases and the historical stack compiler are excluded.

```sh
cargo check --workspace --all-targets
cargo test -p cyber-joy --test execution_claim
cargo test -p joy-rs --test public_execution
cargo test -p cyber-joy --test state_execution
cargo install --path cli --locked
```

Versions in the working branches are development candidates. Public execution
and authenticated public state certificates remain supported; private execution
proofs, full nox proof coverage and live node/database integration remain open.
Historical receipts in [the release audit](audit/release-validation.md) describe
their recorded revisions; current acceptance is recorded in `audit/soft3-only/`.
State commitment v2 changes all BBG roots; regenerate certificates and proofs.

Cyber License: Don't trust. Don't fear. Don't beg.

### Structured native artifacts (0.4 integration)

`joy run-artifact program.dag --input input.dag -o output.dag` executes a complete
ART1 raw-noun program and preserves the output DAG. It emits a JSON execution
receipt after atomic publication; `--force` permits replacing an existing file.
This path uses bounded sequential nox without retaining a trace. See
[the structured-run contract](specs/structured-run.md) for limits and profiles.
Compiler JOB1 admission and proofs of dynamic execution are subsequent gates.
