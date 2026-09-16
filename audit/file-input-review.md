# Bounded file-input review

2026-09-12. Independent review of live Joy/Trisha file-input hardening, not a
new frozen candidate validation. No production files were edited by this review,
no full workspace builds or heavy proofs were run.

## Finding and resolution

The first reviewed Joy version had a real fallback bypass: each artifact
`has_header` returned false on a rejected open/size check, legacy artifact load
errors were ignored during format detection, and `cli/main.rs::load_bundle`
then used unbounded `fs::read_to_string` for `.json` and `.nox`. An oversized
regular file could therefore bypass the artifact read caps; replacing the path
with a FIFO after the initial CLI metadata check could reach a blocking fallback
open. The initial-FIFO CLI test alone did not cover this path.

Reported immediately to the owner. The owner subsequently changed both text
fallbacks to `joy_rs::read_program_text`, which uses the same guarded reader with
a 64 MiB cap. Those call sites and the dedicated exported helper were re-read:
the reported `.json`/`.nox` bypass is closed in live source. The owner is adding
CLI timeout/oversize fallback regressions; their product test receipt is separate
from this read-only review. `.tri` and project directories remain explicit
compiler-source paths; this report does not claim compiler dependency traversal
has the same artifact-file admission contract.

## Reader properties checked

Joy `rs/file_input.rs` and Trisha `cli/input_file.rs::read_regular_bounded`:

- Reject a final symbolic link, FIFO, directory or device at initial
  `symlink_metadata`; reject excessive metadata length before allocating input.
- Open nonblocking and without following the final symlink on released macOS
  and Linux x86_64/aarch64 targets. A regular-to-FIFO race cannot block at open;
  the opened descriptor is then rejected for its file kind.
- Compare opened descriptor `dev`/`ino` against admitted metadata on Unix.
  Replacement by another inode is rejected. A hard link to the same inode is
  the same object, not an identity bypass.
- Recheck descriptor kind/length and read through `take(limit + 1)`, rejecting
  overflow even if a regular file grows after admission. All call-site limits
  are fixed small constants, so the one-byte sentinel cannot overflow.
- These checks do not promise an immutable file snapshot: in-place writes to
  the same inode remain possible. Byte and parser bounds still apply, and the
  actual loaded proof must pass normal format/cryptographic/expected-claim
  checks. Intermediate directory symlinks are not forbidden; `O_NOFOLLOW`
  protects the final component, as the Trisha reference now states.
- Other platforms retain pre/post regular-file checks only. The documented
  stronger race guarantee is correctly limited to the configured platforms.

Header probes still return a boolean and reopen before full parsing. They do
not authenticate a file and must not be relied on for admission by a later
unbounded loader. The repaired text fallback respects this requirement; actual
artifact loaders independently recheck their file and parse its format.

## Exact ABI evidence

| Platform | O_NONBLOCK | O_NOFOLLOW | Evidence |
| --- | --- | --- | --- |
| macOS ARM64 | `0x4` | `0x100` | installed macOS SDK `sys/fcntl.h` and Python runtime constants |
| Linux ARM64 | `0x800` | `0x8000` | actual isolated Ubuntu guest runtime and `aarch64-linux-gnu/asm/fcntl.h` |
| Linux x86_64 | `0x800` | `0x20000` | local upstream libc 0.2.185 `linux/gnu/b64/x86_64/mod.rs`; no native x86 runtime test claimed |

macOS uses the common system fcntl ABI for both configured CPU architectures.
The ARM64 Linux override is essential: blindly copying the x86 no-follow flag
would be wrong. Both implementations contain the correct separate branches.

## Allocation, decoding and proof semantics

Joy public execution files are bounded to 32 MiB; private/state/legacy artifacts
and state certificates to 64 MiB. Modern postcard decoders reject trailing
payload and unsupported artifact format. Legacy compressed assembly now uses
`miniz_oxide::inflate::decompress_to_vec_with_limit` with a 256 KiB output limit;
it does not first inflate an unbounded result. The plain assembly alternative
has the same bound. The existing boundary test covers exactly 256 KiB, one byte
over (compressed to less than 1 KiB), and a valid small formula roundtrip.

Trisha witness/claim/proof transport shares the 64 MiB regular-file reader.
Witness JSON decodes canonical words without retaining a string per field,
bounds each sequence during decoding, and checks the combined public/secret/
digest total of 8 Mi field words. Fixed digest arrays remain exactly five words.
Proof TOML and Base64 allocations are bounded by the admitted transport; they
are not a claim that peak heap usage equals file size. The native SDK still
checks canonical field values, native proof version/shape, claim and proof.
Private malformed input parser errors do not echo the supplied witness content.

These changes restrict file admission and reject oversized representations;
they do not alter the bytes/relations/claim accepted for an admitted valid
proof. This statement is based on inspected diffs and call paths, not a newly
run heavy proof suite. Legacy trace statements still require explicit opt-in
and do not become execution proofs through this hardening.

## Lightweight independent checks

Compiled a standalone harness containing the exact reviewed Joy file helper,
without the temporarily changing product dependency graph. Executed the same
five cases on native macOS ARM64 and the existing isolated Linux ARM64 guest:
valid regular `JOYEXEC2` header/payload succeeds; FIFO, final symlink, directory
and oversized regular file reject. Each invocation had a two-second timeout;
all ten invocations completed. Guest compilation used Rust 1.89.0. This tests
actual file-kind behavior, not a deterministic interposed rename race.

Temporary harness and raw receipts are `/tmp/joy-file-input-review/` on the host
(`host.json`, `linux.json`, copied helper and harness source); guest work is
confined to its matching `/tmp` directory. No source checkout was mounted or
modified, no node process/service or new proof was started. Existing product
integration tests were read for expected coverage; their execution belongs to
the owner after the dependency graph becomes coherent.

## Product validation after the fallback correction

Root's actual public CLI regression passes1 test in0.63s with zero Rust warnings:
`/tmp/joy-artifact-file-tests-reviewed.log`. It covers FIFO and linked-FIFO
verification with an owned process timeout, oversized `.json`/`.nox` fallback
paths, all modern/legacy artifact loaders and state-certificate loading, a fresh
public execution certificate, source-bound and self-contained verification,
wrong-result rejection and final-symlink rejection. A separate legacy assembly
boundary regression passes1 test in0.00s:
`/tmp/joy-legacy-assembly-bounds-reviewed.log`.

The first product compile attempts encountered the concurrently incomplete
experimental Zheng `mod hash` addition. Those failures were preserved in
`/tmp/joy-artifact-file-tests.log` and `/tmp/joy-legacy-assembly-bounds.log`;
the successful reviewed logs above follow the coherent dependency update.

The newly built Joy binary independently verified ten existing FINAL5 artifacts:
five generated on macOS and five on Linux, covering public imported execution,
private execution, typed private entry, public state and hidden state queries.
Expected public input/output and state certificates were supplied. No private
proof was regenerated for this compatibility check. Exact binary/proof hashes:
[verification receipt](file-input-existing-proof-verification.json).
The original installed FINAL5 FIFO timeout remains recorded at
`/tmp/joy-final5-artifact-fifo-before/receipt.json`.
