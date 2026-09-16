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

`--secret` automatically selects `joy-nox-ccs-triton7-zk-v3`; `--zk` selects it
explicitly. Trisha supplies Triton7's randomized STARK for an independently
regenerated checker of the exact Zheng CCS. Private columns are absent from
the artifact. Checked atom calls, fixed-shape branches, arithmetic, words,
structural hashes and equality are supported within the bounded relation.
Inactive branch errors do not invalidate a successful selected path.

`--state state.json` loads a bounded authenticated public BBG certificate.
Public state proofs use `joy-nox-public-state-execution-v1`; each actual lookup
binds namespace, index, returned value and all four root limbs to execution.
Private state proving (`--state ... --zk` or `--secret`) selects cells inside
the proved CCS, hiding query coordinates. It requires all ten public namespaces
and at most2048 total fields; it does not expose private BBG dimensions.

```sh
joy prove private.tri --secret 11 --output private.zheng
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

Use coordinated sibling checkouts of Trident, Trisha, Zheng, Lens, BBG, nox,
Hemera and strata. Bootstrap pinned Triton dependencies first:

```sh
nu ../trisha/patches/apply.nu
cargo check --workspace --all-targets
cargo test -p cyber-joy --test execution_claim
cargo test -p joy-rs --test public_execution
cargo test -p cyber-joy --test state_execution
cargo install --path cli --locked
```

Versions in the working branches are development candidates. The new public,
private and state protocols replace the old state acceptance path; legacy
recursive openings remain refused. Native recursive proofs and installed
public/private/state proof workflows pass on macOS and Linux arm64. Full nox
coverage, live node/database integration, independent cryptographic review and
final coordinated distribution remain release gates. Exact receipts are in
[the release audit](audit/release-validation.md).
State commitment v2 changes all BBG roots; regenerate certificates and proofs.

Cyber License: Don't trust. Don't fear. Don't beg.
