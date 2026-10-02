# Compiler proof census

The ignored `structured::tests::census::driver::frozen_compiler_prefix` diagnostic
admits an unchanged ART1 and JOB1 through Joy's existing admission functions,
then observes bounded pure nox execution. It measures a prefix for choosing a
public proof representation. It supplies no execution proof or SH7/SH8 closure.

The subject is the admitted JOB1 itself. No stage selector or source rewrite is
introduced. The program, formula and subject particles are recorded separately.
Host and admitted limits retain their existing meanings. A separate wall-time
deadline and capture event/byte/work caps may terminate the diagnostic sooner.

The sink counts accepted initial/fresh Node events, transitions, Enter/Return/
Halt/Error actions, decoded entered opcodes, popped/pushed continuation phases,
maximum active invocation depth, completed evaluations and encoded event bytes.
It keeps a bounded invocation stack and fixed-capacity retained dictionaries;
it retains no event vector and writes no trace stream. A transition ceiling
requests cancellation immediately after the accepted transition. An execution
that reaches this ceiling is an unfinished prefix even if a terminal transition
was just observed. Charged execution cost is reported only after successful
evaluator return and a matching Completed event.

The noun dictionary retains the first distinct full particles up to its cap.
Before saturation its distinct count is exact for accepted Node events. Once
an unseen key is refused, the count is a lower bound. Dictionary hits count
repeated events for retained keys; misses after saturation may include repeats.
Retained duplicate headers and cached Costs must agree. Opcode classification
uses retained formula/head headers; missing headers are counted as unclassified.

The evaluation dictionaries separately retain distinct Enter keys and completed
successful keys `(object particle, formula particle)`. A completed entry also
stores result particle and initial-minus-remaining budget; repeat observations
must agree. These observations do not prove budget-independent reuse. Each
dictionary reports capacity, retained entries, observed hits and refused keys.
After any refusal its distinct count is only a lower bound. Open invocations
at cancellation are reported separately from completed evaluations.

The maximum capacities are 262144 noun records, 262144 Enter keys, 262144
completed evaluations and 65536 invocation frames. Runs may tighten these caps.
The explicit prefix ceiling is at most 1000000 transitions; capture is at most
4000000 events, 1073741824 logical bytes and 16000000 work units. The diagnostic
deadline is at most 120 seconds. These are declared ceilings, not measured use.
The JSON output is bounded to 1 MiB and is created without overwriting a file.
No other data file is written by the Rust diagnostic. An external runner limits
process-tree RSS to 2 GiB and captures command output and source/input identities.

Capture delivery-attempt counters remain distinct from accepted sink counters.
Physical allocations, unique particles, evaluation occurrences, semantic charged
reductions, evaluator checkpoints, collection checkpoints and capture work are
reported under their own names. On successful execution, transitions equal
evaluator checkpoints minus the initial checkpoint. Prefix failure retains its
raw failure kind and supplies no fabricated remaining budget or successful cost.

The diagnostic's counters and consistency checks describe host observations.
Authentication, memory birth rules, semantic equations and a completed proof
remain obligations of the future independent Zheng certificate verifier.
