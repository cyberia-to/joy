# Raw nox execution host limits

`Warrior::run`, state-certificate execution and legacy traced execution admit
bracket text through `joy_rs::formula::parse`. Admission uses an iterative parser
and enforces three independent ceilings:

| Coordinate | Maximum |
|---|---:|
| Formula text | 64 MiB |
| Bracket nesting and resulting noun depth | 4096 |
| Syntactic atoms plus pairs | 1,048,576 |

Syntactic nodes count every occurrence before hash-consing. Shared arena storage
therefore cannot bypass text admission. Binary cells and right-nested n-ary cells
have the same noun-depth limit. The existing nox arena can impose a smaller
physical-node limit. Public/ private proving additionally apply their tighter
verifier relation limits.

Successful raw execution returns at most 1,048,576 field words (8 MiB of word
storage). Before flattening a result, Joy counts its expanded leaves with one
memoized traversal of the shared DAG. A result above this ceiling fails before
allocating its flat output. This check is independent of the reduction budget:
a short computation can construct a small DAG representing a very large tree.

The native recursive nox evaluator has its own depth guard. At the pinned nox
revision it rejects evaluator depth above 1000. Joy reserves a 256 MiB worker
stack and retains traces on this raw path; the reduction budget controls charged
execution, while these admission/output limits control surrounding host work.
This in-process API supplies no wall-clock deadline, cancellation or full memory
isolation. The `formula::print` diagnostic helper is recursive and outside the
runtime/CLI execution path; it is intended for trusted small data.

Structured ART1 execution continues to use the separate bounded heap arena,
sequential evaluator and explicit limits defined in `structured-run.md` and
`compiler-jobs.md`. It preserves noun sharing in binary transport instead of
flattening the output to field words.
