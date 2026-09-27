# Native runtime re-audit

Reviewed input: Joy `ea10f8ac28a6e005e9f5c81f6f85aaa0a9692fae`.
Scope: Joy runtime, proof wrappers, state identity, worker admission and
structured ART1/JOB1/RES1 transport. Zheng cryptographic internals are reviewed
separately. This report does not substitute for an independent cryptographic audit.

## Confirmed findings and fix plan

1. R1, medium: `rs/execution.rs:206` confirms a public proof through `self.run`,
   which uses the constructor budget instead of the explicit proof budget.
   `Warrior::with_budget(1).prove_execution(add, empty, 3)` rejects a valid
   three-reduction computation. Use an explicitly budgeted native warrior,
   matching private/state entry points.
2. R2, high for untrusted raw execution: `rs/formula.rs:120` expands shared
   result DAGs without an output ceiling. A 566-byte program with 101 reductions
   creates 1,048,576 output words (8 MiB). Ten further duplications mathematically
   imply 8 GiB; that larger allocation was not attempted. Add a documented
   maximum expanded output size, checked before materializing the flat result.
3. R3, high for untrusted raw execution: recursive `rs/formula.rs:30` has no
   depth admission. A malformed two-million-byte string of opening brackets
   with budget one remains inside parsing; the bounded regression kills its
   child after five seconds. Replace recursion with bounded iterative parsing,
   with named byte, nesting and syntactic-node ceilings.

Initial reproduction command (same input revision plus only new test file):

```
CARGO_TARGET_DIR=../target-runtime-review cargo test -p joy-rs \
  --test reaudit_native --release --locked --offline -- --include-ignored
```

Result: one successful controlled expansion probe and two failing regressions;
`/tmp/joy-runtime-reaudit-probes-bounded.log`. No source fixes preceded this
reproduction. The owner authorized completion of the native implementation;
the parent agent authorized these concrete boundedness and budget fixes.

The fix must preserve binary and n-ary bracket semantics, canonical field atoms,
ordinary compiled programs, exact witness consumption and structured transport.
Tests will cover admitted boundaries, shared DAG expansion rejection and the
explicit budget. No foreign dependency or fallback is involved.

## Implemented correction

The fixes change `rs/formula.rs` and `rs/execution.rs`, with five regressions in
`rs/tests/reaudit_native.rs` and the budget regression in
`rs/tests/public_execution.rs`. The new contract is
`specs/raw-execution-limits.md`. All other runtime/proof call sites inherit the
parser and output checks through their existing calls.

- R1: native public confirmation now constructs `Warrior::with_budget(budget)`.
- R2: count shared output nodes with memoization and reject more than 1,048,576
  expanded words before allocating the flat vector. Scalar output keeps its
  direct fast path. Both binary DAG children must precede their parent;
  malformed/cyclic externally constructed arenas fail explicitly.
- R3: an iterative frame stack replaces recursive raw parsing. Admission checks
  64 MiB of text, 4096 bracket levels/resulting noun depth, and 1,048,576 syntactic
  atoms plus pairs. Occurrences count before hash-consing.

Post-fix evidence, same input revision plus the listed working-tree correction:

```
CARGO_TARGET_DIR=../target-runtime-review cargo test -p joy-rs \
  --release --locked --offline
CARGO_TARGET_DIR=../target-runtime-review cargo test -p joy-rs \
  --test reaudit_native --release --locked --offline
CARGO_TARGET_DIR=../target-runtime-review cargo check -p joy-rs \
  --all-targets --locked --offline
```

The complete library run passed 103 tests with the initial three regression
tests; log `/tmp/joy-runtime-reaudit-fixed-tests.log`. After adding the final
depth-boundary, hash-consing and dynamic-loop cases, the focused run passed all
six with no ignored tests before the budget case was moved into the existing
public-execution suite; log `/tmp/joy-runtime-reaudit-regressions.log`.
The parent run supplies final whole-workspace and installed-CLI evidence.

The nox evaluator itself already checks `MAX_DEPTH = 1000` in
`nox/rs/reduce.rs:64,111`, and both evaluator helpers and dynamic composition
increment depth before recursion. The new dynamic self-application regression
returns the existing malformed-formula error under `DEFAULT_BUDGET`; it neither
exhausts the worker stack nor waits for the whole budget. This is distinct from
the raw parser defect, which previously ran before that guard.

## Twelve-pass coverage

| Pass | Runtime/wrapper result |
|---|---|
| 1. Determinism | Canonical scalar inputs, ordered subjects/witnesses, deterministic parser and state tables; private proof randomization is intentional protocol behavior. |
| 2. Bounded locality | Raw parser/output findings R2/R3 corrected. Structured admission separately charges schema visits, source bytes, modules, diagnostics, transport and evaluator frames. |
| 3. Field arithmetic | Joy delegates field operations to native nox/Goldilocks; public/secret input admission rejects noncanonical words. Raw program atoms retain their existing modular noun semantics. |
| 4. Crypto hygiene | Reviewed private statement/source/name/state binding and explicit public-profile secret refusal; no wrapper acceptance bypass found. Cryptographic core is a separate review. Host secret erasure and constant-time execution are not guaranteed. |
| 5. Types/invariants | `ExpectedJob` and `VerifiedExecution` keep saved expectations and verified coordinates behind private fields. ART1/JOB1/RES1 records and profile numbers are admitted before execution. |
| 6. Errors | R1 corrected; resource failures return errors. No fallback from rejected native proofs to legacy/re-execution exists in these library dispatchers. |
| 7. Adversarial inputs | Tested parser depth/node limits, tiny-DAG output expansion and dynamic recursion. Native private wire has bounded metadata/byte visitors; state tables authenticate before relation construction. |
| 8. Architecture | Joy wrappers use nox/Zheng/BBG/Hemera and Trident boundary types; no foreign proving fallback. Worker adapter owns verification only. |
| 9. Readability | Named admission/output limits, preserved right-nesting semantics and a separate host-limit contract. All changed Rust files remain below the repository file-size ceiling. |
| 10. Compactness | Replaced the recursive parser instead of adding a second runtime parser. Output preflight reuses the arena's DAG order and one memo vector. |
| 11. Performance | Expanded output is preflighted in unique reachable-node work. Raw `run` still collects a native trace; this is an existing memory/performance cost, not foreign dependency evidence. No speed claim is made. |
| 12. Testability | Six independent regression/boundary probes; pre-fix failures retained as evidence, final regressions are enabled. Existing library tests stay green. |

## Intentional boundaries and remaining limitations

No computation-output, cost, state-root or selected-profile substitution was
found in the inspected native wrapper paths. Successful secret execution checks
exact stream consumption. The private-state wrapper authenticates all required
tables, reconstructs the verifier relation from public coordinates, and binds
table leaves and state ABI into its proof statement.

Public stateless artifacts authenticate canonical machine execution; their
descriptive name/source compilation are not cryptographically certified. This
is documented behavior, and the worker adapter also compares the exact saved
assembly. Private/state artifacts bind their additional declared identity.
None proves compiler semantic preservation.

Job ID and attempt associate transport with saved expectations. Current proof
statements do not make a computation fresh for a job/network/height; cancellation,
trusted-root freshness, durable acceptance and rewards remain caller policy.
This is explicit in `specs/worker-adapter.md`, not a missing Neptune dependency.

Secret-provider vectors, caller input and native trace/arena buffers are not
universally zeroized, and native execution can branch on secrets. The proof
profile protects its serialized witness under its protocol assumptions; this
review makes no host-memory, side-channel or independent-cryptographic-audit
claim. Public program, output, reductions and state tables remain disclosed.

`formula::print` is a recursive public diagnostic helper used only by current
tests, not by runtime/CLI execution. It remains suitable for trusted small
nouns. Raw execution still reserves a 256 MiB worker stack and has no hard
process-memory or wall-clock isolation. Structured execution is the bounded
transport/evaluator path for large compiler jobs.
