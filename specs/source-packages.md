# Exact source files to JOB1

`joy pack-job --compiler COMPILER.dag --manifest PACKAGE.json -o JOB.dag
[--force] [LIMITS]` serializes source files into the canonical JOB1 consumed by
`run-artifact`. It performs no source lexing, parsing, import discovery, module
header checking, typechecking or code generation. The manifest supplies the
complete source set, including standard/VM modules needed by the guest compiler.

Manifest version1 is selected explicitly by its required `version: 1` field:

```json
{
  "version": 1,
  "entry_module": "sample",
  "entry_function": "main",
  "modules": [
    {"logical_path": "sample", "file": "sample.tri", "origin_name": "local", "origin_version": "1"}
  ],
  "options": {"target": 0, "input_profile": 0, "output_profile": 0, "optimization": 0, "cfg_flags": []},
  "limits": {
    "source_bytes": 4096, "modules": 32, "diagnostics": 16,
    "sequence_length": 4096, "validation_visits": 100000,
    "artifact_bytes": 1048576, "artifact_nodes": 3000, "artifact_depth": 128,
    "reductions": 1000000, "arena_nodes": 3000, "evaluator_frames": 16384
  }
}
```

All fields are required; unknown fields and versions reject. Paths in `file`
are relative to the manifest directory unless absolute. They never enter JOB1
or its identity. Module records are sorted by logical path; cfg flags are sorted
by ASCII bytes. Duplicates reject. Origin labels and exact source bytes enter
the noun unchanged; no Unicode/newline normalization or implicit cfg flags apply.
Invalid UTF-8 and embedded zero in sources remain legal package bytes.

The compiler must be a complete valid ART1 with machine0 profiles1/1. Its full
root identity becomes JOB1.expected_compiler. Manifest/compiler reads use the
same bounded regular-file policy as structured execution. The sum of source
reads is bounded by requested source_bytes; module count and all requested
limits are checked against independent worker ceilings before opening modules.

LIMITS are the same independent host flags/defaults as `run-artifact`, including
compiler schema caps. A package request cannot raise them. The pack worker uses
the requested lifetime arena allowance and a cooperative deadline for construction
and validation; file reads remain bounded regular reads without hard preemption.
The initial compiler/manifest reads and JSON deserialization precede the worker
timer. Temporary JSON allocations are bounded by the host manifest byte cap;
module/sequence counts are admitted after deserialization. Construction is
bounded by source sizes and counts, with tree depth at most32. Full production
JOB1 admission runs before encoding/publication, including requested compiler and
job transport caps and shared validation visits.

The host writes complete canonical NOXDAG01 without recursive DAG expansion.
Equivalent file bytes/labels/options/limits and compiler identity produce
identical JOB1 bytes regardless of manifest/module order, host paths or timestamps.
Changing a source byte, origin version or explicit option changes the job identity.

Success atomically publishes only JOB1 and prints `joy/job-pack/v1`, its path,
compiler/job/package/module/source particles, entry/options/limits and admission
visits. This receipt establishes packaging, not guest compilation or a proof.
Failure exits1, writes no success receipt and preserves an existing output even
with `--force`. The guest owns source declarations, reachable import closure and
all language diagnostics after packaging.

Packing and structured execution share the physical heap-arena policy in
[structured run](structured-run.md). An explicit host allowance above 196608
selects the larger arena; manifest limits still must fit the host and tighten
the same lifetime allocation counter. Changing only the physical capacity with
the same manifest preserves JOB1 bytes. Changing manifest LIM1 changes JOB1 and
RES1 identities, while a successful extracted ART1 remains capacity-independent.
