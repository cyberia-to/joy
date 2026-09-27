# Bounded finalizer reuse in structured execution

`run-artifact` now uses nox's pure, execution-local finalizer cache. The worker
reports its fixed 1,048,576-byte buffer separately. Existing input admission,
logical arena allowances, charged reductions, frame limits, cancellation
checkpoints and atomic output publication remain unchanged. The cache contains
no compiler or source-language operations.

The [validation receipt](validation.json) records repository revisions, local
integration changes and commands. The complete Joy release test suite passed
162 tests; workspace/all-target checking emitted no warnings. The resolved
soft3 dependency gate passed with 145 packages and no foreign compiler features.
This is a local feature-branch integration build, not a release candidate.

The [nox comparison](../../../nox/audit/sequential-untraced/README.md) measured
the same fixed C1/JOB1 workload before and after caching. Both executions halted
with identical propagated budget, nodes, frames and cancellation checkpoints.
Their elapsed times were 57.437 and 16.090 seconds. A propagated child halt
budget is not a root remainder; those failed runs do not report exact gas use.
Nox's successful and failing differential fixtures establish cost and allocation
equivalence separately.

Full compiler self-hosting remains an independent gate. This change accelerates
the worker without increasing its admitted gas or arena ceilings.
