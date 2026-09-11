# Joy

Interface specification: [Joy CLI](specs/cli.md). Node integration and
acceptance are owned by [cyber's worker contract](../cyber/specs/worker.md).
Both specs distinguish the implemented baseline from the planned interface.

Joy compiles and executes Trident programs on nox. Supported public programs
can also produce a Zheng execution certificate whose input, output, program
and reduction count are checked by the verifier.

```sh
joy run program.tri --input-values 3,5
joy prove program.tri --input-values 3,5 --output program.zheng
joy verify program.zheng --claim 24 --input-values 3,5
joy verify program.tri --proof program.zheng --claim 24
joy verify program.tri --claim 24 --input-values 3,5  # native re-execution
```

`prove` uses `zheng-nox-public-execution-v1`. Zheng derives a global CCS from
the canonical program, authenticates the complete public witness and checks
all constraints and public coordinates. Verification does not run nox.
Hashing, arithmetic, comparisons, word operations and bounded compiled
imports/loops/branches are covered within the supported symbolic surface.

The certificate reveals the full witness and has linear verification cost.
It is **not zero knowledge or succinct**. Proving with `--secret`, calls,
state reads, dynamic continuations or unsupported branch shapes fails;
ordinary `run --secret` remains available. Both branch arms must have defined
arithmetic even when unselected. See [Zheng's exact contract](../zheng/specs/execution.md).

Public inputs bind as `[p_last [... [p_first 0]]]`. Proof verification checks
requested `--claim` and `--input-values`; `--budget` bounds the certificate's
budget. Self-contained artifacts carry the program; passing a source/bundle
with `--proof` also checks that the certificate belongs to that program.

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

Use coordinated sibling checkouts of Trident, Zheng, Lens, nox and soft3:

```sh
cargo check --workspace --all-targets
cargo test -p cyber-joy --test execution_claim
cargo test -p joy-rs --test public_execution
cargo install --path cli --locked
```

Versions in the working branches are development candidates, not new
published releases. Authenticated state execution, zero knowledge, deployment
and complete nox support remain unfinished. Three legacy state-proof acceptance
tests still fail explicitly; passing public execution tests does not hide them.

Cyber License: Don't trust. Don't fear. Don't beg.
