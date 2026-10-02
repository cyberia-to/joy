# Compiler certificate transport validation

Source base: Joy71b327eb40813ef19073b4952b6e3b7d8bc7f94c. Exact changed-file
SHA256 values and every pinned sibling revision are in `final/source.json`.
Commands, Rust1.89 identity, raw logs, exit codes and warning scans are retained
in `final/receipt.json`. This is transport component evidence, before production
semantic dispatch or whole-compiler proofs.

The final source passed the complete release workspace suite, all-feature /
all-target compilation and the soft3 dependency boundary gate. The final test
log includes seven transport tests and an isolated allocation-refusal test.
They exercise empty/raw/compressed/multiple frames, exact and one-below caps,
every truncated prefix and single-byte mutation of a fixture, reordered and
foreign frames, self-consistent malformed compressed lengths, appended streams,
bounded decompression bombs, short reads, partial writes and poisoned sessions.
The separate allocator executable refuses actual payload allocations and checks
that initialized writing/compression allocates no further buffers.

`review.json` records independent review. Its allocation finding changed the
writer from the convenience compressor to the bounded low-level compressor.
The pinned compressor's two fixed startup allocations remain explicitly
documented; verifier payload allocations return an allocation error. The first
replacement build used private re-export paths for TDEFLFlush/TDEFLStatus and
failed with E0603; imports were corrected to `deflate::core` before the passing
final gates. Initial pre-review passing logs/source remain at this directory's
root and are not relabeled as final-source validation.

The framed bytes authenticate transport order and context only. Acceptance of
an execution also requires the planned Zheng semantic records, complete public
artifact bindings and a valid terminal result. These tests close none of SH7,
SH8 or the cross-platform distribution gates.
