# Independent structured certificate review

Scope: Joy's public certificate dispatch, expected-artifact admission, context
digest, semantic record codec, Session, verification/result pipeline, and
transport endpoints. The reviewer changed only
`rs/structured/certificate/tests.rs`, its `tests/` children, and this audit
directory. No production correction was needed and no commit was made by the
reviewer; the owner integrates the complete delivery.

`source.json` identifies reviewed production/specification bytes, independently
written tests and clean sibling dependency revisions. The starting Joy revision
is `bd91dd6281714ea794672ba83c26bf7bf12d1bad`; this audit applies to the listed
uncommitted source hashes on top of that revision. The source bytes did not
change during the retained final gates. `commands.json` records full commands,
environment, UTC starts, elapsed times, return codes and log hashes.

## Results

Both commands use Rust 1.89.0, `--locked --offline --release`, jobs=2, and the
isolated `certificate/target-review` target directory named in the command log.

| Command | Observed result | Raw evidence |
| --- | --- | --- |
| `cargo test -p joy-rs --lib structured::certificate::tests` | 15 passed, none failed or ignored | `independent-tests.log` |
| `cargo test -p joy-rs --lib structured::certificate::` | 31 passed, none failed or ignored | `certificate-family.log` |

No compiler warning appears in the acceptance logs. Filtered tests are excluded
from these counts. The family run includes existing transport and separately
authored producer/capture tests; this review does not take authorship or a full
independent source-review claim for capture or the Nox observer.

Exploratory logs are retained. `initial.log` exposed a no-op test mutation:
replacing the branch test's zero result with zero is valid. `extended.log`
exposed a malformed test fixture: atom zero was used where an empty SEQ1 was
intended. Both fixtures were corrected; `compiler-fixtures.log` then passed its
13-test intermediate set. Two further epoch/eviction tests are included in the
15-test final set. No production defect was hidden by these fixture fixes.

## Independent construction

The raw fixtures construct ART1/NOXDAG values and quote, add, compose and both
branch derivations directly. They do not call the Joy producer or Nox evaluator.
The compiler fixture constructs only axis-zero, quote and cons rules to produce
RES1 from a schema-built JOB1; it constructs a result container, not a compiler.
Its quoted ART1 and arbitrary source are test data, with no self-host claim.

Every semantic attack is serialized through the actual record codec and a new
transport Writer, recalculating compression, context and frame chain. Therefore
a corrupted semantic relation must be rejected after valid frame authentication.
The tests separately check byte-level truncation and trailing input.

Covered cases:

- Every Enter coordinate and Finish result in the hand-authored derivations,
  removed/reordered children, extra roots, selected-arm substitution, and both
  computed-continuation coordinates.
- Every limb of expected program/formula/input particles, profile, budget and
  frame allowance; changed caller artifacts; a changed input with a freshly
  recomputed matching transport context and an unchanged derivation.
- Initial reset, strict next epoch, positive bounded snapshot count, incomplete
  snapshot, canonical atoms, strictly prior pair indices, stale current-table
  IDs and complete remapping between every continuation phase.
- Empty/out-of-range/stale cache handles, full-key mismatch, actual slot eviction,
  reuse after noun reset, expanded metrics and exact/one-below resource caps.
- Session poisoning, all terminal particle limbs, low/high cost changes,
  independent decoded result identity, partial/extra terminal payload, underlying
  transport truncation and trailing bytes.
- Valid compiler result extraction; semantic success with wrong RES1 job
  identity, generated profile, status or empty diagnostics; changed JOB1 source
  with freshly recomputed context; invalid JOB1 and wrong compiler identity.

## Review conclusion

No correctness finding in the source scope identified by `reviewed_files`.
Passes 1 through 12 were considered with the following concrete boundaries:

The deterministic statement binds complete particles, admitted profile, budget
and frame allowance. Clock readings affect cooperative stopping and observational
reporting. ART1/JOB1 admission obtains the context independently of wire metadata.
Session allows only bounded current noun tables and the opaque Zheng stream;
epoch replacement retains no semantic noun indices. Every failed record leaves
the Session poisoned. Its record cap covers reset/noun churn and terminal, while
Zheng accounts complete expanded metrics on cached reuse with checked integers.

The codec has fixed-width fields and rejects unknown tags. Result bytes are
bounded, independently decoded and compared to the checked terminal particle.
Compiler RES1 admission then checks job identity, status, diagnostics and ART1
profile. Immediate transport completion and EOF reject unused payload and
trailing bytes. Public APIs validate administrative caps before worker dispatch.
The public witness model and unattested physical-resource report are explicit.

A tentative concern about the verifier retaining initial program/input storage
was examined and withdrawn: the pinned compacting runtime also retains the
entire initial arena prefix. No failing resource-equivalence case was found or
claimed. Standard allocator failure policy in prover maps and compression state
is documented; this review adds no new claim of complete allocation-fault coverage.

This review relies on the previously reviewed Zheng noun/semantic components and
existing ART1/JOB1/RES1 schemas. It does not independently repeat all primitive
cryptanalysis, attest physical LIM1/GC/deadline use, validate release platforms,
or establish whole-compiler certificate capacity or SH7/SH8 completion. CLI
publication/fresh-process gates and producer/observer reviews have separate owners.
