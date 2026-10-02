# Bounded compiler proof census

This diagnostic observes the frozen C2 compiler and unchanged S1 JOB1 through
Joy admission and the nox semantic observer. It retains bounded dictionaries
and an invocation stack, with no stored event stream. These are host prefix
observations for choosing a certificate representation. They establish no
execution proof or SH7/SH8 acceptance.

The code base is Joy `77fec58bd39f9e9cb1e957590831109439352e45` plus the exact
five-file source maps in each `*-command.json`. The final source map is in
`workspace-command.json`, `check-2-command.json` and both `prefix*-2-command.json`
receipts. The sibling maps retain their revisions, resolved paths and clean
statuses, including nox `172811b7746cdcd6ab198a3ed976dc6c19f55d5b` and Zheng
`633e5ba980db94ca10d9d6b67cf24d9d3aef63a7`. No dependency source was edited.

The compiler SHA256 is
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`;
the JOB1 SHA256 is
`3474e5583e7b3ac36bdd526435bb2ae584691774a009e29ca02407c22589f13d`.
The receipts bind the full input paths and byte sizes before and after each run.
Both admit the same 94 modules and 370544 source bytes. The program particle is
`2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe`,
and the unchanged JOB1 subject particle is
`b9454107f6296f21febd6e68dd98b1b108cc31e195acdad7fe1fb9246f422d36`.

## Measurements

Commands ran from the isolated Joy worktree. `run.py` retains the complete
Cargo arguments, environment, wall times, input/source hashes, raw stdout/stderr
hashes and sampled process-tree RSS for each label. RSS is sampled every 250 ms;
the supervisor kills its own process group if a sample exceeds 2 GiB. Sampled
peaks are observations, not exact peak-memory bounds. The Rust diagnostic
declares the capture, retained-record, invocation and output-file ceilings in
[the contract](../../specs/compiler-proof-census.md).

| Value | `python3 audit/compiler-proof-census/run.py prefix100k 2` | `python3 audit/compiler-proof-census/run.py prefix1m 2` |
|---|---:|---:|
| Accepted transitions | 100000 | 1000000 |
| Enter occurrences | 50339 | 500175 |
| Successful completed evaluation occurrences | 49661 | 499825 |
| Distinct Enter keys | 42911 exact | at least 262144 |
| Enter hits in retained dictionary | 7428 | 219046 |
| Enter keys refused after capacity | 0 | 18985 |
| Distinct completed evaluation keys | 42233 exact | at least 262144 |
| Completed-key hits in retained dictionary | 7428 | 219079 |
| Completed keys refused after capacity | 0 | 18602 |
| Initial noun events | 176875 | 176875 |
| Fresh noun events | 9076 | 52570 |
| Distinct noun particles | 185951 exact | 229445 exact |
| Repeated noun events / collections | 0 / 0 | 0 / 0 |
| Peak active invocations | 688 | 1007 |
| Open invocations at stop | 678 | 350 |
| Logical encoded event bytes | 49446232 | 290716880 |
| Trace bytes written | 0 | 0 |
| Admission/setup microseconds | 2087331 | 2098158 |
| Observed execution microseconds | 140778 | 719296 |
| Sampled peak process-tree RSS, KiB | 334720 | 424416 |

The larger evaluation dictionaries saturated at their declared capacity. A
refused key can occur repeatedly, so refusal counts are not additional unique
keys. Arithmetic from the recorded counters places distinct Enter keys in
262144..281129 and distinct completed keys in 262144..280746 for this prefix.
These intervals and retained hits do not measure a complete compiler DAG.
Neither prefix includes collection, and neither completes the compiler.
Successful cost, remaining budget and result fields are therefore null.
The raw stop is `Execution(Execution(Cancelled))`; capture counters remain
separate from evaluator/collection checkpoints and physical allocations.

The first prefix runs are retained as well. After adding the terminal-boundary
fixture and including the output newline in the byte cap, both were rerun on
the final source. Every non-time JSON field matches between each original and
final prefix run. Timings are single local observations, not benchmarks of
certificate generation or verification.

## Checks

| Command label | Result |
|---|---|
| `format-2`: explicit-file `rustfmt --check` | passed |
| `boundary`: `python3 scripts/check-soft3-boundary.py` | passed |
| `check-2`: `cargo check --workspace --all-targets --release --locked --offline` | passed; zero warnings |
| `workspace`: `cargo test --workspace --release --locked --offline -- --test-threads=2` | 179 passed, 0 failed, 1 ignored; zero warnings |
| `prefix100k-2`, `prefix1m-2`: explicit ignored census invocation | passed; zero warnings |

The workspace count sums its test binaries once. The ignored test is the
explicit frozen-input diagnostic; its separate runs are not added to that
count. Seven small census fixtures cover repeated evaluations, dictionary
saturation, ordinary and terminal prefix cancellation, continuation/depth
agreement with unobserved execution, atomic retained-header rejection and
capture-limit failure. The final full suite includes them.

The initial focused run failed one reporting assertion: cancellation at the
next evaluator checkpoint produced `Execution(Execution(Cancelled))`, while
the test expected `Capture(Cancelled)`. The retained `focused-1.log` and command
receipt show the failure. The corrected tests preserve both raw failure forms:
terminal Return at the prefix cap can cancel delivery of Completed, and still
cannot publish a successful result. No evaluator behavior was changed.

## Toolchain and installed binary

The postcommit installation at Joy
`51da51797f2af1a8acf8f053143c7a86fe53f455` passed with zero warnings; its exact
command, PATH, binary hash and raw output are retained in
`postcommit-install.json`, `.stdout` and `.stderr`. It installed only into this
isolated census family. The resulting binary reports `joy 0.5.0` and has SHA256
`886c1edaa7372ef3a0d04af53d4d6b46723e48199977f31935a236e66bd03f28`.

`toolchain.json` records the resolved executables and hashes: Homebrew Cargo
1.95.0 at `/opt/homebrew/Cellar/rust/1.95.0/bin/cargo` and rustc 1.95.0 at
`/opt/homebrew/Cellar/rust/1.95.0/bin/rustc`. The retained Cargo compiler cache
records the matching rustc version and sysroot. Earlier `run.py` receipts did
not capture PATH or RUSTUP variables; the supplement labels their current
observation as retrospective rather than inventing contemporaneous evidence.
