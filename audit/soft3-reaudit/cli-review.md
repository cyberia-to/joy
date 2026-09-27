# Independent Joy CLI and dependency-boundary review

Baseline sources: Joy `ea10f8ac28a6e005e9f5c81f6f85aaa0a9692fae`,
Trident `c15f328d8265ef128e26008f0d31076d5a0cfb45`.
Review scope: `joy/cli/`, `joy/scripts/check-soft3-boundary.py`, and delegated
filesystem reads reached through the compiler API. Native execution/proof
internals have separate reviewers; this review makes no complete cryptographic
or production-readiness claim.

## Confirmed findings

### CLI-A1 — medium: compiler source loading can block before execution admission

Baseline locations: `joy/cli/compile.rs:12,46,87,111`,
`trident/src/config/project.rs:62`,
`trident/src/config/resolve/resolver.rs:47,112,121`.

A `.tri` FIFO passed to `run` or `build`, or a FIFO `trident.toml` in a project,
blocks in unbounded `read_to_string`. `--budget 1` does not bound that work.
The same FIFO with a `.nox` extension is rejected immediately by Joy's hardened
artifact reader. This is an availability/input-admission defect, not acceptance
of a false proof. Imported filesystem modules and manifest reads share the
underlying unbounded reader.

The baseline executable was built with:

```text
CARGO_TARGET_DIR=../target-cli-review cargo build --release --locked --offline -p cyber-joy
```

Reproduction on the baseline executable:

```python
import os, subprocess, tempfile
from pathlib import Path
joy = "/Users/master/cyber/.worktrees/joy-soft3-reaudit/target-cli-review/release/joy"
with tempfile.TemporaryDirectory() as root:
    root = Path(root)
    os.mkfifo(root / "pipe.tri")
    # TimeoutExpired after 2 seconds; subprocess.run kills its own child.
    subprocess.run([joy, "run", "pipe.tri", "--budget", "1"],
                   cwd=root, capture_output=True, timeout=2)
```

Observed baseline cases: `run pipe.tri --budget 1`, `build pipe.tri`, and
`run project --budget 1` with FIFO `project/trident.toml` each reached that
2-second test deadline. `run pipe.nox --budget 1` exited 1 with a regular-file
admission error. The test-created inputs and children were removed afterward.

Authorized correction is compiler-owned bounded regular-file UTF-8 loading,
including descriptor checks and a capped descriptor read. Source symlinks remain
supported when their target is a regular file. The concrete scope and limits
are recorded in `joy/.claude/trident-wiring.md` and
`trident/reference/cli.md`. This bounds each filesystem read; aggregate project
size, parsing work and proof execution have their own contracts.

### CLI-A2 — medium: batch changes the meaning of option-looking path values

Baseline locations: `joy/cli/batch.rs:92-95,117-121,202`.

`flag` forwarded valued options as separate arguments. A legal parent invocation
using `--input-file=--witness.json` becomes child arguments
`--input-file`, `--witness.json`, and Clap treats the value as another option.
The same defect affects an output directory supplied as `--output=--proofs`.
The child fails rather than running the requested workload; no shell parsing or
proof-authentication bypass was found.

Baseline fixture: `ok.nox` contains `[1 7]`; `--witness.json` contains
`{"schema_version":1,"public":[],"secret":[]}`. Commands:

```text
joy run ok.nox --input-file=--witness.json
joy batch run ok.nox --input-file=--witness.json
joy batch prove ok.nox --output=--proofs
```

Observed baseline: direct run exits 0 and prints `7`; both batch invocations
exit 1 with child `unexpected argument` diagnostics. The fix creates one native
`OsString` per valued flag (`--flag=value`), preserving non-UTF-8 values without
putting private file contents in argv.

## Twelve review passes

| Pass | CLI/boundary evidence |
|---|---|
| 1. Determinism | Batch captures/emits results in input order. Build publication has deterministic payloads; proof randomness and benchmark timings are intentional, distinct outputs. |
| 2. Bounded locality | Witness files, job count, parallelism and retained child output have explicit caps. CLI-A1 was the unbounded filesystem-loading gap. Per-file compiler limits do not claim an aggregate graph bound. |
| 3. Field arithmetic | The CLI validates canonical unsigned words and modulus bounds; arithmetic remains with native owners. Benchmark expectations use the same admission. |
| 4. Crypto hygiene | File parse errors suppress parser text and private values; batch passes a file path. Inline `--secret` exposes argv by documented design. CLI buffers are not comprehensively zeroized and execution is not claimed constant-time. No new private-proof fallback or secret diagnostic leak was found. |
| 5. Types/invariants | Typed Clap arguments, strict witness fields/duplicates and canonical Word decoding constrain input. CLI-A2 crossed the typed parent/child argv boundary incorrectly. |
| 6. Errors/cleanup | Temporary output cleanup is RAII; failed proving precedes publication. Batch aggregates child failures. `process::exit` is spread through command modules, an existing deviation from the compiler style guide rather than a new correctness defect. |
| 7. Adversarial input | Reviewed native/public/state header dispatch, retired-header refusal, file limits, links/streams, claim expectations and publication paths. Source FIFO defect is explicit above. |
| 8. Architecture | Joy calls the compiler API and native owners; no subprocess/compiler/prover fallback to a foreign warrior. The all-feature Cargo gate succeeds. |
| 9. Readability | Per-command modules and profile names expose the verification meaning. Platform open flags have ABI comments; Linux aarch64 constants were cross-checked against local libc source. |
| 10. Compactness | Private and public artifact dispatch remain separate. File-admission code is duplicated between CLI witnesses and library artifacts; no refactor is required to fix the findings. |
| 11. Performance | Batch caps retained output per stream and invokes the same executable with bounded process count. This does not bound native flattening/parser work; those paths are covered by the runtime review. |
| 12. Testability | Added independent tests for option-looking/native-byte argument values, exact witness-file byte admission and symlink publication. Compiler descriptor-race and source/project transport regressions cover CLI-A1. |

## Boundary evidence and negative findings

At the baseline, `python3 scripts/check-soft3-boundary.py` reported:

```json
{"ok":true,"packages":145,"compiler_features":[],"command":["cargo","metadata","--format-version","1","--locked","--offline","--all-features"]}
```

The deny-list and compiler-feature check cover the known foreign VM/prover
packages and embedded external compiler targets. This is an executable assertion
about the resolved graph; it is not proof that every conceivable future foreign
package name is prohibited.

Native private dispatch precedes public/state/legacy fallback and rejected
artifacts return failures. Retired `JOYZK001/2/3` are refused before legacy
inspection. Legacy inspection rejects requested IO/state/secret constraints.
Public execution compares canonical program semantics; stronger source/build
identity binding belongs to the private/state contracts. This difference is
already explicit in `joy/specs/cli.md` and is not reported as a forgery.

The project-name POSIX traversal hypothesis was refuted:
`trident/src/config/project.rs::validate_project_name` rejects path separators
and parent traversal. No platform-native Windows adversarial execution was
performed by this reviewer; cross-compilation is tracked by the root review.

## Focused validation

Corrections are based on the source revisions above. Batch code is committed as
`8563864`; the final Joy CLI regressions and wiring plan are committed as
`e2e95f92bb6ff1616f619c1e82d8e61157f2c545`. Current local commands and logs:

- `CARGO_TARGET_DIR=../target-cli-review cargo test --release --locked --offline -p cyber-joy --test reaudit_cli`: 3 passed on macOS; `/tmp/joy-reaudit-cli-tests.log`.
- `CARGO_TARGET_DIR=../target-cli-review cargo test --release --locked --offline -p cyber-joy --bin joy batch::tests`: 1 passed; `/tmp/joy-reaudit-batch-unit.log`.
- The invalid-UTF-8 filesystem-name integration is Linux-only: macOS rejected creation of that fixture with `Illegal byte sequence`. Native OsString byte preservation is separately exercised on macOS by the unit test.
- Trident correction committed as `6e9fb45ff814ecd4c873e895f875e4730e93d184` on `fix/0.4-bounded-source-loading`. `CARGO_TARGET_DIR=../target-cli-review cargo check --locked --offline --all-targets` passed with no Rust warning lines; `/tmp/trident-reaudit-source-check.log`.
- At that compiler tree, `CARGO_TARGET_DIR=../target-cli-review cargo test --release --locked --offline --lib config::text_file::tests --test source_files --test build_context --test cli_check_diagnostics --test import_scope` passed the 4 helper unit tests. The shared filter selected no integration tests in this first command; `/tmp/trident-reaudit-source-tests.log`.
- The subsequent unfiltered `CARGO_TARGET_DIR=../target-cli-review cargo test --release --locked --offline --test source_files --test build_context --test cli_check_diagnostics --test import_scope` passed 14 integration tests with no Rust warning lines; `/tmp/trident-reaudit-source-integration.log`. Negative compiler fixtures emit expected diagnostics.
- Joy batch correction committed by root as `8563864`. With the final Joy test tree now committed as `e2e95f9` and compiler `6e9fb45`, `CARGO_TARGET_DIR=../target-cli-review cargo test --release --locked --offline -p cyber-joy --test reaudit_cli` passed all 4 macOS tests without Rust warnings; `/tmp/joy-reaudit-cli-source-tests.log`.
- No full slow self-hosting suite or cross-target checks were duplicated by this reviewer. Root owns final workspace, cross-platform, installed integration and C1 artifact comparison gates.

The initial filesystem-name test failure was a fixture portability issue; it
was retained in this account and corrected by declaring its Linux platform
requirement. Existing historical audit reports remain unchanged.


## Disposition and remaining bounds

Both confirmed CLI findings have corrections and focused passing regressions.
Source admission is now owned by Trident rather than a Joy check followed by an
unbounded compiler reopen. The opened regular descriptor is the descriptor read;
source symlinks remain supported and replacement/growth checks are exercised.

The 4 MiB source and 1 MiB manifest/lockfile caps are per-file byte bounds.
They do not add a total module-count/source-package budget, a compiler deadline,
or complete parser nesting/AST allocation quotas. Caller-supplied in-memory
source overlays retain their existing API semantics. Those limitations must
remain explicit when assessing untrusted compilation as a service.
