# Native compiler job admission

Protocol owner: `trident/reference/self-hosting-jobs.md`, with collections from
`trident/reference/self-hosting-data.md`. This document specifies Joy's worker
policy and CLI integration. It is the implementation contract for SH1.

`run-artifact` dispatches only exact ART1 machine0 profiles raw(0,0) and
compiler(1,1). Compiler input is a complete JOB1. Admission validates exact
records, full compiler identity, positive supported LIM1, package/module/cfg
order, names, origin labels, source lengths, canonical collections and entry
presence. It preserves source bytes without parsing them. Import resolution,
declared module checking and compilation remain guest work.

The existing transport/runtime limits are independent host ceilings. Additional
host caps, available as CLI flags, are:

| Flag | Default | Hard maximum |
|---|---:|---:|
| --source-bytes | 4194304 | 16777216 |
| --modules | 4096 | 65536 |
| --diagnostics | 1024 | 65536 |
| --sequence-length | 65536 | 1048576 |
| --validation-visits | 1000000 | 16777216 |

All are positive. Every LIM1 field must fit its host ceiling. After extracting
LIM1, already consumed input validation visits count toward the requested cap.
Compiler and JOB1 containers must also fit requested transport limits. The
arena's lifetime allowance is tightened to the request; already loaded nodes
remain charged. Evaluation uses exactly requested reduction and frame limits.
The host deadline covers admission, execution and result validation cooperatively.

Input and output validation receive separate visit allowances. Every checked
record charges head/tail reads, tag and terminator reads; each scalar/digest
read is charged. A collection wrapper charges its structural/scalar reads.
Balanced tree traversal charges each logical visited subtree once, including
repeated shared subtrees; a canonical empty subtree is checked by its particle.
Occupied Seq leaves are returned whole. Each Bytes word is checked/read once
during that traversal and expanded into at most four bytes. This host accounting
is intentionally distinct from guest-library and SH0 reference traversal costs.
No allowance is reset per record. The receipt reports actual input/output visits.
Host vectors grow only within admitted sizes and charged traversal; no expanded
untrusted noun tree or copied module source text is retained after admission.

Output must be RES1 bound to the exact admitted JOB1. A successful payload must
be ART1 matching requested target and profiles. A diagnostic payload must be a
nonempty bounded Seq of sorted DIA1 records, with valid UTF-8 messages, known
codes, package indices and source byte spans. Code8 may occur once, at the entry
module's empty start span. Duplicate ordinary diagnostics remain permitted.
Complete RES1 and extracted ART1 obey requested transport limits. Runtime faults
and malformed results return errors without any output publication.

Library `RunResult.output` remains the exact complete execution result. Its
optional `compiled` bytes contain only the validated guest-produced ART1.
`RunReport.compiler_job` records package/module identities, entry, options,
requested limits, validation visits, compile status, diagnostics and optional
program identity. Existing program/input/output identities bind compiler/JOB1/RES1.
No host language stage repairs or augments the guest's program.

CLI `run-artifact --emit result|program` defaults to `result`. `result` publishes
the complete output (raw noun or RES1); a valid compile-error RES1 is a successful
execution with `compiler_job.status = compile_error`, exit0. `program` requires
successful compiler-profile execution and publishes the extracted ART1. For a
valid compile-error result it exits1 with guest diagnostics and leaves the old
destination intact. A raw-profile run cannot use `--emit program`.

Exactly one selected file is atomically published. JSON success is printed only
after publication and distinguishes the published particle from the RES1 output
particle. `--force` preserves the old file on admission, runtime, result or
publication failure. These receipts certify observed execution, not a proof or
self-compilation. The transport test compiler need not parse any source; SH2
separately requires compilation inside nox.
