# Structured native artifact execution

Status: 0.4 implementation contract. Joy owns complete noun transport and
execution; Trident owns ART1 and native source lowering; nox owns NOXDAG01 and
bounded sequential pure L1 evaluation.

`joy run-artifact PROGRAM --input INPUT --output OUTPUT [--force] [LIMITS]`
loads two complete NOXDAG01 files. PROGRAM must have exact
`ART1(0,0,0,formula)` layout for arbitrary raw INPUT nouns, including atom0,
or `ART1(0,1,1,formula)` for validated compiler jobs. See
[compiler-job admission](compiler-jobs.md) for JOB1/RES1, independent schema
caps and `--emit result|program`. This path never parses source,
flattens nouns, invokes the old flat-word Runner ABI or falls back to a prover.
Reached host call/look services are rejected by the sequential pure profile.

Limits (each positive, each with an independent hard ceiling):

| CLI flag | Default | Worker ceiling |
|---|---|---|
| --budget | 1000000 | 100000000 |
| --arena-nodes | 196608 | 786432 |
| --frames | 16384 | 65536 |
| --artifact-bytes | 16777216 | 16777216 |
| --artifact-nodes | 196608 | 196608 |
| --artifact-depth | 4096 | 4096 |
| --time-ms | 30000 | 60000 |

Transport limits apply independently to each program/input/output container.
Loaded program and input share one arena and lifetime node allowance with all
execution allocations. Hash-cons sharing is charged once. Requests up to 196608
nodes select a 262144-slot arena; larger explicit requests select 1048576 slots.
Both use nox's fallible heap constructor, initialized in place. The logical
allowance stays exactly as requested; choosing a physical capacity never raises
it. The default remains 196608. Packing JOB1 and executing ART1 use the same
capacity selection. Allocator failure returns an error before publication.

The arena reserves its full `size_of::<Reduction<N>>()` storage, separately from
the 256 MiB worker stack. `arena_reserved_bytes` reports that arena storage, rather
than the size of its owning pointer. Frames and codec workspace have their own
allowances. Artifact byte/node ceilings remain unchanged: lifetime intermediate
allocation and exported DAG size are separate bounds. Frame-buffer byte accounting
comes from nox's actual Frame size. Traces are not
retained: NoTrace execution reports successful charged reductions as budget
minus remaining; errors never fabricate cost or produce a successful receipt.

The cooperative deadline starts inside the worker before decoding. Check it
between input/output codec stages and every evaluator transition. Join the
worker on every completion/failure; no timed-out detached thread may publish
later. File reads occur before that timer and are bounded regular-file reads;
allocator, individual codec stages and filesystem writes are not preemptible.
This is not a hard process-wide elapsed-time or RSS guarantee. Arena, frame,
file and codec caps bound the admitted work/storage separately.

On success encode the complete output first, then publish atomically through
an exclusive staging file. Default refuses overwriting any destination; --force
explicitly permits atomic replacement. Execution/export/publication failure
preserves existing destination contents and removes staging files. Report
JSON only after successful publication: schema joy/artifact-run/v1, ok, artifact
path, program/input/output particles, charged_reductions, allocated_nodes,
peak_frames, arena_reserved_bytes, frame_buffer_bytes, worker_stack_bytes,
elapsed_micros and trace_mode none. These are execution observations, not proof.
Failure exits1 with a diagnostic; Clap syntax errors exit2. Existing commands
and ProgramBundle public/secret input conventions retain their own contracts.

Library API `structured::run` accepts owned program/input bytes and RunLimits;
`run_files` performs bounded file loading. Both return output bytes and a report,
without publishing. The CLI alone performs publication. Limits above are worker
policy and do not redefine the nox or Trident protocol integer ranges.

## Compile source to ART1

`joy build SOURCE --emit artifact [--artifact-profile raw|compiler-job] [-o PROGRAM.dag] [--force]`
uses Trident's native artifact API. Source has exactly `fn main(input: Noun) -> Noun` and
may import `vm.nox.noun` and ordinary modules. The result is the complete
NOXDAG01 ART1 consumed by `run-artifact`; no bracket text or flat
input adapter is involved. Imports, target selection and named project profiles
use the same resolution as other build modes. Host services and legacy flat
I/O declarations are rejected in this profile.

The output defaults to `source.dag`, or PROJECT_NAME.dag for a project directory.
Encoding completes before atomic publication; `--force` is required to replace
an existing file. `--format json-v1` reports the usual build envelope with
format `artifact`, the full `program_particle`, byte length, declared entry/result profiles,
compiler/target-package identities and selected profile. The default artifact
profile is raw(0,0). Explicit `--artifact-profile compiler-job` sets both ART1
profiles to1; the same source ABI and pure lowering apply. `--artifact-profile`
with a different emit format rejects before publication. Selecting compiler
profile declares the JOB1/RES1 boundary; Joy still validates both records when
executing, and the selection does not certify compiler behavior. This format's execution
identity is the ART1 particle; it does not contain a bundle source hash.

This is seed compilation on the host. Guest compiler JOB1/RES1 uses the separate admission path above. Native dynamic
proofs remain a separate gate.
