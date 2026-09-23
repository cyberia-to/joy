# Raw source build delivery — 2026-09-23

`joy build --emit artifact` now resolves a source project and emits the exact
ART1 raw profile consumed by `run-artifact`. It uses Trident's complete native
artifact API and the same target/project/profile resolution as bundle builds.
Encoding finishes before the existing atomic publisher sees output bytes.
Overwrites require --force; compile failure preserves the previous file.

Three new CLI integration cases cover native SDK/helper imports, source/API/
project artifact equality, named profiles, target precedence, complete nested
output and preservation on type/profile/publication errors. Full workspace:
88 passed, zero failed/ignored. The all-target workspace check is clean.
Existing native, state and Triton-backed private proof tests also pass.

[Observed CLI receipts](raw-source-observations-2026-09-23.json) bind the full
program/input/output identities and compare exact bytes against independent
hand-authored fixtures. Source addition returns 14 in 12 reductions; identity
preserves [[1 2]3] in 6 reductions; the Field literal p returns 0 in 6 reductions.
These sources are host-seed compiled. Guest compiler JOB1/RES1 and self-hosting
are still open. No new proof capability is advertised.

[Validation manifest](raw-source-validation-2026-09-23.json) records local logs
and sources. Branch target is release/0.4; main is unchanged.
