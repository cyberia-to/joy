# Explicit compacting host deadline

The explicitly selected compacting deadline ceiling is four hours. Its default
remains 30 seconds and ordinary execution retains the five-minute ceiling.
The production delta is `COMPACT_TIME_MS`; program/JOB1 identities, certificate
context, logical budgets and all storage/proof allowances remain unchanged.

The original native Linux full-proof run [36961998100](https://github.com/cyberia-to/trisha/actions/runs/36961998100)
failed at its selected two-hour deadline. Its complete failure evidence is
retained in Trisha commit `ec05b133efe3fa71c28d4e50e5defd0f6be8150d`, under
`audit/whole-self-build-linux-20261002/`. This change requires a separate profile
and source pin for a new run. It supplies no successful whole Linux proof result.

## Actual validation

`python3 -B audit/host-proof-deadline/run.py focused check test boundary` ran
with the absolute Rust1.89 toolchain against base Joy
`11b7bad201cbfc03955290b5479fa6684f8e2b82` plus the exact source identities in
each receipt's matching `sources_before` / `sources_after` manifests. The patch
and new CLI deadline test are retained with this commit. Sibling revisions are
recorded individually; the shared Cargo target is a warm build cache.

- Focused structured CLI suite: 10 passed, 0 failed, 0 ignored. The new test
  compares three independently produced certificate byte streams and all nine
  fresh producer/verifier deadline combinations, then checks both commands'
  rejection of zero, one above the ceiling and u64 maximum while preserving
  existing destinations and removing unpublished staging files.
- Full release workspace: 216 passed, 0 failed, 1 existing ignored census test,
  across 29 result records. RunLimits boundary/cancellation and collected
  compiler execution preserve the original output, costs and defaults.
- All-target release check: passed with warnings denied.
- Resolved soft3 boundary: passed, 145 packages, no compiler default features.

Raw commands, tool versions, logs, source identities and sampled resource
receipts are in the four gate directories. `run.py` bounds its own process
group to 1800 seconds and 6 GiB sampled RSS, preserving failure evidence.
`independent-review/` records separate code/spec/test review; `contract-review/`
records why the physical deadline is outside the authenticated logical claim.
Post-commit installation is recorded separately after this source commit.

## Post-commit installation

`python3 -B audit/host-proof-deadline/run.py install` built and installed source
commit `2151e2bd67ca7ea4ae19ee4c8ca8ec5a93c4a881` successfully, but the audit
wrapper marked it failed because Cargo warned that the fresh destination was
absent from PATH. The original receipt/logs remain in `install/`.

The corrected wrapper places that explicit destination on PATH;
`python3 -B audit/host-proof-deadline/run.py install-path` passed with the same
source, actual Rust1.89 and zero warnings. The original and corrected runs use
a warm Cargo target and preserve all tested production files. `install-path/`
records the installed binary identity. This installation confirms packaging of
the source change; full Linux proof validation remains separate.
