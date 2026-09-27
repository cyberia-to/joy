# Native witness files and batch commands

Joy's input schema version 1 carries native scalar public and secret words:

```json
{"schema_version":1,"public":["7"],"secret":["35"]}
```

`--input-file PATH` on run/prove and re-execution verification conflicts with
`--input-values` and `--secret`. All three fields are required; unknown and
duplicate fields fail. Arrays accept JSON unsigned integers or canonical
unsigned decimal strings, each below the Goldilocks modulus. Decimal strings
have no leading zeros except `0`. Inputs contain at most 65536 words total.
The file must be regular, at most 4 MiB, and must not be a symbolic link.
Diagnostics omit the path, parser contents, and secret values. A private input
file belongs to the caller; Joy never copies its contents into child argv.
Proof verification receives public expectations; private witnesses are required
only for execution and proving.

```text
joy batch run INPUT... [--input-file PATH] [--max-parallel N]
joy batch prove INPUT... --output NEW_DIRECTORY [--input-file PATH] [--zk] [--max-parallel N]
joy batch verify PROOF... [--max-parallel N]
```

Run and prove accept shared `--input-values`, `--target`, `--profile`,
`--budget`, and `--state`. Secret batch inputs use `--input-file`. Batch verify
accepts shared target and budget. Each command accepts 1–256 inputs, with
1–8 concurrent isolated child processes (default 2). Children invoke the exact
current executable and the ordinary run/prove/verify command directly, without
shell parsing. Child stdout and stderr are emitted in input order. Each stream
is captured up to 4 MiB; exceeding that cap makes the job fail.

Prove creates a new output directory exclusively, with a numbered subdirectory
for each input and `proof.zheng` inside it. Identical input basenames therefore
cannot collide. Existing output directories are refused unless `--force` explicitly permits
replacement of numbered proof outputs. Symbolic links are refused. Successful jobs remain
available if another job fails. Child failure, launch failure, or output-limit
failure makes the whole batch fail; diagnostics identify the zero-based input
index. Batch preparation errors occur before any child is launched.

These commands preserve the selected native proof profile and its resource
limits. Running jobs in parallel does not aggregate their proofs or establish
network admission. A compatible cyber adapter keeps network policy and custody
with their existing owners.

## Native tests

```text
joy test SOURCE_OR_PROJECT [--target nox|cyber] [--profile NAME]
```

Joy passes the normal source/project/profile resolution to Trident's native
`run_tests` API. The compiler selects active `#[test]` functions from the AST,
including imported modules and profile/test cfg flags. Each test executes on
nox in an isolated arena with a one-million-reduction budget. Assertions,
invalid tests, compile failures and runtime failures cause a nonzero exit;
compilation alone does not count as a successful test. Test functions require
no parameters, type parameters or return type. A project with no active tests
reports that fact. Application `main` is not executed as a substitute.

## Native execution measurements

```text
joy bench INPUT [--input-values WORDS | --input-file PATH] [--claim WORDS]
                [--repeat N] [--budget N] [--state CERTIFICATE]
                [--target nox|cyber] [--profile NAME]
```

The command compiles or loads the exact supplied program once, then executes
it 1–1000 times (default 5) through Joy's normal native warrior. Every execution
gets a fresh witness stream and the same immutable inputs and optional public
state certificate. Outputs and charged reductions must agree across all runs.
An explicit `--claim` checks the independent expected output on every run.
Any mismatch or execution failure fails the whole measurement.

Successful stdout is one JSON `joy/bench/v1` document. It identifies the program
and records each actual execution's reductions and elapsed nanoseconds, plus
program loading/compilation time, total/minimum/median elapsed time, output and checked state
root. Quantities that may exceed JavaScript precision use decimal strings.
The program particle uses Hemera over trimmed nox assembly and records that
profile explicitly. Compilation profile is null for an already compiled bundle
or raw formula. For even sample counts the median is the upper middle sample. Timings include
native runtime preparation and state-certificate validation performed inside
the normal execution call; no warmup or calibration is silently subtracted.

`expected_output_checked` distinguishes a supplied expected result from repeated
consistency alone. `proof_generated` is false: these measurements execute code
and do not produce cryptographic proofs. The command neither generates a
reference baseline nor claims comparisons against unexecuted implementations.
