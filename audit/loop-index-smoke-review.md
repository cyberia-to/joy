# FINAL4 loop-index smoke regression

The source strings in `rs/tests/loop_indices.rs` were checked byte-for-byte
against the existing FINAL4 installed-smoke sources. The historical archive and
smoke directory were not modified.

- `release_helper.tri`: `c5d2e49afbfc5896403eba9fcca4ea84c076678a6c543111d7e44022d301db67`
- `imported-nox.tri`: `dfbade31bff4ef28259c974f43213567c04a33ee3a58377c0e4e69670cfba0e1`

`cargo test --release --locked -p joy-rs --test loop_indices --
--test-threads=1` passed **2 tests**, zero failed/ignored/Rust warnings,
in **15.68 seconds**. Log: `/tmp/joy-final4-loop-indices-proofs.log`.

The exact fixture produced [803] from [3,5] and [809] from [4,5] in both
source profiles. All four cases generated and verified public execution
certificates and genuine Triton ZK execution proofs, including serialization
roundtrips. Changes to either public input or public output were rejected.
The fixture has no secret input; the ZK backend hides its execution witness.

The independent scope regression also generated four public certificates for
nested loops with the same index name, an absorbed conditional local shadow,
and early returns: flag0 returns7, flag1 returns109 in both profiles. A nearer
runtime index binding correctly fails compilation rather than inheriting a
loop/module constant.

Read-only inspection found no additional defect in the changed Scope logic:
`bind` clears constant metadata, `lookup_constant` stops at the nearest binding
including None, loop frames restore outer bindings, and sealed absorbed frames
clear names, constant metadata, and types together. Both ordinary-transform and
returning-loop paths use `bind_loop_index`; indices outside U32 are refused.
This establishes the bounded/unrolled index fix, not support for arbitrary
runtime-index array access on nox.
