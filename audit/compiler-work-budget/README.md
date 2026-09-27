# Explicit compiler work allowance

Local development evidence on Joy parent `62dadd35abe83b3d6dea77fe5f60baac050530e9`.
[validation.json](validation.json) records the tested source hashes, sibling
revisions, exact commands, log hashes and preserved evidence.

The owner contract in [structured-run.md](../../specs/structured-run.md) permits
an explicitly selected compaction reduction allowance up to 20,000,000,000.
The existing `--budget` flag selects it; JOB1 LIM1 must also admit the requested
allowance. The ordinary 100,000,000 ceiling and every default stay unchanged.
The independent resident, cumulative-allocation, collection-work, frame,
deadline, transport and compiler-schema limits stay unchanged. No retry or
compiler-specific execution path is added.

The preserved [full compiler failure](full-run-baseline.json) records the
original full-source C1 run failing at its configured 10,000,000,000 allowance.
Failed charged reductions are unknown. Its artifact, JOB1 and binary hashes,
command, diagnostic and available collector counters are retained verbatim.
This ceiling change does not reinterpret that failure as successful work.

The preserved [prefix receipt](stage120.json) reports 3,146,170,244 reductions
through admission, discovery, ordering and the first 21 dependency-ordered
module bodies. It is a pure VM diagnostic, not Joy admission or C2 acceptance.
Subtracting the successful discovery cost of 627,648,529 and scaling the
remaining work by source bytes gives the explicit projection
`627648529 + (3146170244 - 627648529) * 369707 / 97778`, about 10.15 billion
reductions for all bodies. The static source model, discovery receipt and
their commands/revisions/hashes are identified in `validation.json`.
Module costs need not scale with bytes, and generation is absent from this
projection. The new 20-billion ceiling permits a bounded follow-up with room
to investigate both phases; it does not establish a sufficient allowance.

All 170 workspace tests pass. Focused tests admit the exact new ceiling and
reject one above it, retain acceptance at the previous ceiling, and compare
small-run output, charged reductions, allocation and collection counters
under the default compacting allowance and explicit 10/20-billion allowances.
Compiler fixtures verify that a JOB1 cannot enlarge a smaller host allowance
and still tightens a larger host allowance. These are admission and semantic
checks; they do not execute a 20-billion workload or establish self-compilation.
All-target checking has zero Rust warnings. The resolved soft3 boundary
contains 145 packages and no Trident foreign-target features.

Saved logs remove only trailing whitespace and terminal empty lines. Original
copies remain outside the repository, with original and normalized SHA256s in
the receipt. No previous audit or native compiler fixture quota was changed.
