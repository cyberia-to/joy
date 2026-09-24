# Explicit heap arenas for compiler jobs

Source: Joy `820041bed4a7d649721a2d25b7ac92d162860854`, nox
`c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trident
`1e08dedbb0ec7b2dd25fc54ecc16c0587f8fc1ab`.
[Pinned owner validation](heap-compiler-arena.json) includes all sibling pins,
commands, observations and failed drafts.

`pack-job` and `run-artifact` construct nox arenas directly on the heap. Host
allowance selects a trusted capacity; JOB1 limits still tighten the same lifetime
allocation counter. The default remains 196608 nodes, with an explicit ceiling
of 786432. [The policy](../../specs/structured-run.md) keeps artifact limits
independent and reports the full reserved arena representation.

`cargo check --workspace --all-targets --locked --offline` and
`cargo test --release --workspace --locked --offline` pass with zero Rust
warnings and 122 tests. Transport fixtures verify byte identity across tiers;
these quoted fixture compilers are separate from the real compiler acceptance.

From the pinned Trident checkout, `python3 audit/self-hosting/run-native-arena.py
--joy ../install/bin/joy --output /tmp/native-arena-cli.json` records 41 commands
and 10 observations. The real compiler now handles 31 assignments and 64 nested
calls beyond the default quota. Exact/one-below small and large JOB quotas run
on the large physical arena, and failed jobs preserve the previous file. The
same JOB keeps its result and program bytes across physical tiers. The existing
242-command compiler corpus also passes with prior program identities intact.

A 4096-byte invalid-UTF8 source reaches a guest diagnostic; a valid source
padded to 4096 bytes still exhausts the larger allowance. Full compiler scale,
self-build, six-platform release and native Zheng compiler proofs remain open.
See [Trident's measured report](../../../trident/audit/self-hosting/native-compiler-arena.md).
