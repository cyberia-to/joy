# Native Joy correctness and size re-audit

Date: 2026-09-27. Host: macOS arm64, Homebrew Rust 1.95.0.
Scope: native Joy execution, public/private/state proof dispatch, saved worker
expectations, installed CLI, compiler filesystem admission, resolved dependency
and embedded-resource boundary, and physical binary size.

The native stack works in the exercised cases without Trisha, Triton or Neptune.
This review found and corrected resource-admission and CLI defects. Local private
execution still has side-channel and memory-erasure limitations. This is local
development evidence; it does not promote a release or certify the cryptography.

## Findings and corrections

| Finding | Severity / consequence | Correction |
|---|---|---|
| Explicit public-proof budget ignored during native confirmation | Medium; valid API calls failed with a smaller constructor budget | Use the explicit budget for both relation and native execution; `c076c6f` |
| Shared result DAG expands without a flat-output ceiling | High for untrusted raw jobs; tiny programs can demand huge output allocation | Memoized expanded-leaf preflight, maximum 1,048,576 words; `06cb2f1` |
| Recursive raw parser runs before VM limits | High for untrusted raw jobs; malformed input exhausts parser resources | Iterative parsing with byte, syntax-node and actual noun-depth limits; `06cb2f1` |
| Compiler opens FIFO source/project input | Medium; CLI blocks before execution budget applies | Compiler-owned regular-file and bounded reads, including imported files and project package metadata; Trident `6e9fb45` |
| Batch splits option-looking values into child options | Medium; direct commands work but equivalent batch calls fail | Preserve each valued argument as one native `--flag=value` token; `8563864` |
| Secret-dependent zero-inverse shortcut | Medium local timing exposure with identical public output/cost | Always compute field inversion, including zero; Zheng `deb5b8d` |

Details and before/after reproductions:
[runtime review](runtime-review.md), [CLI review](cli-review.md), and
[Zheng proof review](../../../zheng/audit/soft3-reaudit/proof-review.md).
The CLI compiler change belongs to Trident, with the wiring request recorded in
`joy/.claude/trident-wiring.md`.

Two privacy tasks remain open: activity/branch/table-access timing throughout
private preparation and native execution, and complete erasure of secret witness,
PRG, arena/trace and caller-owned copies. Removing the measured inverse shortcut
does not make the engine constant-time. Public program, output, reduction count
and complete state certificate tables are intentionally disclosed. No accepted
false execution proof or witness disclosure through the inspected proof transcript
was found; absence of a found attack is not a security proof.

Raw execution retains traces and reserves a 256 MiB worker stack. Its limits
bound specific work, not process RSS or wall-clock time. Structured compiler jobs
use the separate bounded sequential evaluator. Proof coverage is the documented
static relation subset; unsupported continuations/shapes fail explicitly.

## Frozen baseline and attribution

All size/profile measurements below use the committed baseline Joy `ea10f8a`,
Trident `c15f328`, Zheng `2a9e9cc`. Exact sibling revisions and clean input status
are in [baseline-sources.json](baseline-sources.json). A separate detached sibling
workspace preserved these inputs while fixes were made in the audit workspace.

Build the baseline with this command from that Joy workspace:

```sh
CARGO_TARGET_DIR=../target-baseline cargo rustc -p cyber-joy --bin joy --release --locked --offline -- -C link-arg=-Wl,-map,/absolute/path/joy-link.map
python3 audit/soft3-reaudit/measure-binary.py --binary /absolute/path/joy --map /absolute/path/joy-link.map --trident /absolute/path/trident --output baseline-size.json
```

The measured binary is 4,910,992 bytes; SHA-256 and raw attribution are in
[baseline-size.json](baseline-size.json). Its only dynamic dependencies are
macOS `libiconv.2.dylib` and `libSystem.B.dylib`, observed with `xcrun otool -L`.
System libraries are outside this on-disk executable, just as for the native
Rust comparison below. Virtual `__PAGEZERO` is not a four-gigabyte file payload.

| Physical component | Bytes | Interpretation |
|---|---:|---|
| `__text` | 2,732,764 | Machine code |
| Unwind and exception sections | 544,468 | `__gcc_except_tab`, `__unwind_info`, `__eh_frame` |
| `__TEXT,__const` | 685,504 | Read-only constants, including embedded language sources |
| `__LINKEDIT` | 831,376 | Linker metadata, symbol/string tables, signature and related records |
| Other sections, headers and padding | 116,880 | Remaining physical file bytes |

The linker map assigns all machine-code bytes to containing object archives:

| Object archive | Code bytes |
|---|---:|
| Trident compiler | 1,110,412 |
| Joy library | 439,760 |
| Joy CLI objects | 353,772 |
| Clap argument parser | 252,592 |
| Rust `std` | 204,420 |
| Zheng | 180,440 |
| Other archives | 191,368 |

These are object ownership figures. Inlining and instantiated generics mean they
are not exact semantic ownership or the bytes saved by removing a crate. For
example, some nox reduction code is instantiated in Joy's objects. A dependency
count is not a binary-size attribution.

Embedded source content accounts for 559,702 unique physical bytes across 118
files, already included in the constants above. Of that, 87 native nox compiler
modules contain 324,154 bytes. The seven retired stack-compiler source files are
absent. Shared standard-library source belongs to the compiler SDK; this is a
complete compile/run/prove/verify tool, not a specialized Hello World executable.

The all-feature resolved Cargo graph contains 145 packages and zero compiler
external-target features. `python3 scripts/check-soft3-boundary.py` passes.
Installed symbol and embedded-resource checks independently reject foreign
packages/targets. The audit workspace has no Trisha sibling.

## Controlled build-profile experiments

No shipping profile change is made by this audit. All rows keep the same feature
set and panic unwinding. The baseline workspace has no release profile override;
[Cargo's documented defaults](https://doc.rust-lang.org/cargo/reference/profiles.html)
include optimization level 3, 16 codegen units and no cross-crate LTO. Size modes
must be measured; they need not produce the fastest or smallest result universally.

| Profile | Original bytes | After `strip -x` and ad-hoc signing |
|---|---:|---:|
| Existing release defaults | 4,910,992 | 4,292,032 |
| Thin LTO, one codegen unit, opt 3 | 4,117,984 | 3,732,352 |
| Fat LTO, one codegen unit, opt 3 | 3,906,336 | 3,495,232 |
| Fat LTO, one codegen unit, opt s | 3,391,392 | 2,921,088 |

Stripped sizes in this table come from the installed-run reports, with the same
destination filename `joy.stripped`; signature identifiers can slightly change
file size for other filenames. Build commands/hashes are in
[size-profiles.json](size-profiles.json). `strip -x` removes local symbol metadata,
preserving panic behavior and all features. No abort-on-panic shortcut was used.

Each profile passed the same installed validation script (51 commands including
provenance queries and packaging checks); reports:
[baseline](baseline-installed.json), [thin](thin-installed.json),
[fat](fat-installed.json), [small](small-installed.json). The reports distinguish
the fixture runner's current workspace from the frozen binary's source revisions.

`compare-profiles.py` runs one warm-up, seven measured private-product proofs and
verifications, and three measured native C1 compiler builds, alternating order.
All compiler artifacts have the same SHA-256. Raw commands/samples and exact
binary hashes are in [profile-performance.json](profile-performance.json).

| Profile | Prove median ns | Verify median ns | Compile C1 median ns |
|---|---:|---:|---:|
| Existing defaults | 213,639,250 | 101,535,792 | 3,292,394,042 |
| Thin LTO | 214,013,292 | 102,616,709 | 3,348,402,292 |
| Fat LTO, opt 3 | 216,078,042 | 101,780,875 | 3,336,950,083 |
| Fat LTO, opt s | 217,814,958 | 103,032,500 | 3,334,830,750 |

These samples show comparable elapsed time for two workloads on this development
machine. They do not establish a universal regression bound or other-platform
performance. Fat LTO plus stripping is the conservative next packaging candidate;
opt s is worth testing across realistic proof workloads before adopting globally.

For comparison, native Rust `println!("Hello, world!")` on this same toolchain
occupies 431,488 bytes with release-like defaults and 286,080 with fat LTO,
opt s and symbol stripping; [source, commands and hashes](rust-hello.json).
The Trident Hello World nox formula is 292 bytes. A distributable Joy-plus-formula
pack must include the full chosen Joy binary, giving approximately 4.91 MB with
current defaults or 2.92 MB in the measured small profile, plus the formula.
Archive compression and shared system-library disk usage are not included.

## Delivery validation

Final source revisions, commands and their results are recorded in
[validation.json](validation.json):

- Joy: 162 release tests passed, zero ignored/failing tests; all-target native
  `cargo check` and the resolved dependency gate passed without compiler warnings.
- Zheng: 66 execution tests and three new audit tests passed. The timing
  diagnostic was run separately; it is intentionally ignored in normal suites.
- Trident: four descriptor/transport unit tests and fourteen compiler integration
  tests passed; standalone all-target check passed without compiler warnings.
- Rebuilt and installed the committed implementations; the installed validation
  script passed ([51 recorded commands](final-installed.json)), and the real
  Trident/Joy private run/prove/verify chain passed ([seven commands](final-paired.json)).
- Current corrected Joy: 4,910,272 bytes, or 4,292,096 after stripping/signing.
  The controlled optimization table above uses the frozen pre-fix baseline.
- The corrected compiler produced exactly the same 9,191,495-byte C1 artifact
  with SHA-256 `4aed7fc83be96156fcb65c3bbb369c192ad27f894ab78a030e3588056a66d112`.
- [Cross checks](cross-checks.json): macOS x64, Linux arm64/x64 and Windows x64,
  using rustup Rust 1.98.0. These are compilation checks, not execution tests;
  native runtime and size experiments used macOS arm64/Homebrew Rust 1.95.0.

The fixes are delivered on feature branches targeting `release/0.4`: Zheng
[PR #36](https://github.com/cyberia-to/zheng/pull/36), Trident
[PR #108](https://github.com/cyberia-to/trident/pull/108), and the Joy runtime/CLI
audit branch. No default branch, version tag or published
release is changed by this work.
