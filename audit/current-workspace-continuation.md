# Current workspace continuation — 2026-09-12

This receipt concerns the live Joy workspace after the bounded artifact readers
and scalar-helper formal analysis changes. Frozen FINAL5 sources and binaries
are unchanged. It is separate from the root's frozen 198-proof baseline run.

## Commands and execution scope

From `joy/`:

```sh
CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=4 cargo test --workspace --release --locked -- --test-threads=1
CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=4 cargo check --workspace --release --locked --all-targets --all-features
```

The first command includes ordinary small proof tests; no ignored tests or
additional experimental proofs are requested. Logs are
`/tmp/joy-current-workspace-continuation.log` and
`/tmp/joy-current-workspace-all-features.log`. Final outcomes are recorded below
only after completion. Resource observations are sampled RSS, not a claim of
measured peak resident memory.

## Read-only tagged production integration assessment

Inspected `rs/execution.rs`, `rs/zk_execution.rs`, `rs/zk_state.rs`,
`zheng/rs/src/execution/{statement,private,tagged/mod}.rs` and
`zheng/specs/props/tagged-symbolic-relation.md`.

A direct replacement of `PrivateStatement::prepare` is unsound as a protocol
migration. Existing `ExecutionStatement.public_output` stores flattened leaves;
its fixed-shape relation supplies the missing topology. The tagged relation
allows different output trees under the same matrices, so its statement must
bind the complete output noun. Existing native `Warrior` returns leaves too.
A new adapter needs the actual native output noun for its independent prover
check, while preserving ordinary `ExecutionResult.output` compatibility.

The smallest sound production addition is a separate, explicitly versioned
**stateless tagged execution protocol**, with these changes:

1. Zheng defines a bounded canonical tagged statement/preparation API. Program
   and output use validated noun tokens, not an unbounded recursively decoded
   tree. Public inputs retain Joy's reversed cons-list plus terminal zero;
   subject topology follows public input count, never a prover-selected shape.
   Input values, full output topology/payload, exact selected cost and budget
   are validated canonically. The verifier derives matrices solely from the
   canonical formula and that subject topology.
2. The transcript binds protocol and tagged-kernel revision, canonical program,
   subject ABI, complete input/output noun, cost, budget and resource policy.
   Joy additionally binds expected source/build identity as its current private
   artifact does. Matrix generation and carrier allocation must be deterministic
   for this revision; no fallback on witness-dependent compilation outcomes.
3. Adapt `public_coordinates` with sorted, consistent duplicate merging.
   Coordinate zero must equal one and is independently enforced by Trisha's
   `Checker`; the zero coordinate, every input, output tag/payload/padding and
   cost must remain pinned. Never discard inconsistent duplicate pins.
   `Checker` already binds the exact matrices and transcript bytes in its
   program, and the full public coordinates in its native STARK claim.
4. Joy adds a distinct bounded artifact/header and explicit dispatcher, plus
   prover-side native noun/cost comparison. Canonical decode rejects trailing
   data, oversized token counts/depth and field aliases before allocation
   amplification. Proof verification regenerates the checker without native
   execution or trust in supplied matrices. Existing `JOYEXEC2`, `JOYST001`
   and `JOYZK003` verification semantics remain unchanged.
5. A CLI/API selector and capability metadata expose the actual added subset.
   A rejected call/state/computed-continuation relation must never be retried as
   a weaker proof format. Old private calls and authenticated state requests
   continue through their existing explicit protocols.

This is a genuine subset expansion, not complete general-nox support. Tagged
currently rejects calls/state and computed formula continuations except quoted
static composition. Private/state integration requires additional constrained
activity-dependent call cursors, bounded private witness consumption, exact
call-pattern semantics and BBG root/namespace/key/value/table authentication.
In particular a state lookup in an inactive branch must not consume a private
query or alter the active query schedule. Public tables, inclusion/root checks
and `root_in_subject` bindings from the current private-state path cannot be
replaced by an unconstrained returned value. Full computed continuation support
needs the separately reviewed bounded evaluation/memory relation; it is not
provided by this adapter proposal.

## Required adversarial acceptance gates

- Same regenerated matrices/checker program for atom/pair branches, but genuine
  proofs reject changed inputs, reversed pair children, different association
  with identical leaves, tag/payload/padding edits and changed cost or budget.
- Rewrite the complete envelope public claim to another valid branch: STARK
  verification still rejects, rather than only a metadata comparison failing.
- Bind constant one against all-zero homogeneous witnesses; reject omitted,
  duplicate-conflicting and out-of-range public coordinates.
- Inactive native failures produce no obligation, active malformed projections
  or invalid word operands fail; unsupported opcodes never silently disappear.
- Old public/private/state artifacts retain positive and hostile regression
  coverage; new tagged artifacts cannot impersonate any old header/domain.
- Native and relation results agree on complete nouns and exact cost, including
  imported compiled programs and limit boundaries; no flattened-output-only
  oracle. Test parser limits before any large allocation.

The experimental `trisha/rs/tests/tagged_ccs.rs` currently provides one actual
VM test (both atom/pair witnesses) and one compiled ignored genuine proof test.
That is useful preparatory evidence, not a production protocol implementation
or a completed general-nox release gate.

## Completed release test receipt

The full release workspace test command exited **0**: **71 passed, 0 failed,
0 ignored**, across 16 result summaries (including zero-test/doc-test summaries),
with no warnings. This includes the actual private typed-entry, loop-index and
terminal-return proofs, plus the bounded reader regressions. Sampled RSS was
3,755,424 KiB for the loop-index proof process and 9,886,528 KiB for typed entry;
these are observations, not a continuous peak measurement. Both processes
completed normally. No extra or ignored proof was launched.

The release `--all-targets --all-features --locked` workspace check also exited
**0**, with no warnings. This is compile coverage of every target/feature,
not an additional all-features execution suite. No production defect was
observed in these invocations; no production files were changed by this review.
