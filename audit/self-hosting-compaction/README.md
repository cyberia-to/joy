# Explicit bounded compaction in Joy

Local development evidence. Nox `1eaa8a494f994e4da6b20509469c1e62624f3f2d`
provides the collector. Joy's parent is `4e814a9`; exact implementation file
hashes, sibling revisions and commands are in [validation.json](validation.json).
The full compiler's C2/self-build result is a separate acceptance gate.

The new policy is opt-in through `--resident-nodes` and `--collection-work`.
The existing `--arena-nodes` allowance charges cumulative fresh allocations in
this mode; loaded nodes remain pinned and charged. Old defaults, ceilings and
error text remain unchanged when compaction is omitted. Both raw ART1 execution
and compiler JOB1/RES1 use the policy. Job quotas still tighten host allowances.
Packing is bounded by resident storage, without collection or language stages.

The [installed CLI receipt](installed-loop.json) records the exact binary hash
and three commands on the existing `loop4097` fixture. The 256-node append-only
run fails. Explicit compaction returns the expected complete noun and charges
61460 reductions, with 8198 peak frames. It performs 76 collections, reclaims
16112 nodes and peaks at exactly 256 resident nodes. Cumulative allocations
are 16344, final resident count 232, collection work 45551664 units and scratch
1049600 bytes. These are counters for that fixture, not general compiler bounds.
A collection-work failure preserves the previous output even with `--force`.

The workspace gate passes 168 tests, including repeated-collection noun/gas
equivalence, exact cumulative/work frontiers, loaded-node rejection, service
and frame/budget failures, actual JOB1/RES1 binding and extracted artifact
validation after collection, CLI flag pairing and failed-publication guards.
All-target checking has zero Rust warnings. The resolved soft3 boundary has
145 packages and no Trident foreign-target features.

Nox's separate [collector audit](../../../nox/audit/sequential-compaction/README.md)
contains differential phase/root, cancellation/index and accounting tests,
plus the exact saved full-source discovery comparison. Traced execution and
proof APIs retain their previous policy. No foreign VM fallback or compiler
language stage is introduced into the worker.
