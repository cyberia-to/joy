# Native computation verification adapter

`joy_rs::worker` is a typed library boundary for a scheduler that retains its
own computation expectations. Cyber owns the job lifecycle and admission policy
specified in `cyber/specs/worker.md`. This adapter verifies a produced native
artifact against an immutable saved job, without command-output parsing.
It does not open node storage, hold keys, dispatch jobs or change network state.

## Saved expectations

Construct `ExpectedJob::new(JobSpec)` before worker dispatch. The job owns its
exact compiled `ProgramBundle`, job ID, attempt, selected `ProofProfile`, ordered
public inputs, optional expected output, maximum reduction budget, maximum
artifact bytes and optional trusted state root. The constructor validates native
target compatibility, canonical fields, program syntax and declared limits.
`ExpectedJob` exposes a shared read-only reference to the saved specification.
Private witnesses have no field in the job or produced result types.

Supported selections:

| Profile | Artifact | State requirement |
|---|---|---|
| Public | `zheng-nox-public-execution-v2` | no state context |
| PublicState | `joy-nox-public-state-execution-v1` | exact trusted BBG root |
| Private | `joy-nox-zheng-private-execution-v1` | no state context |
| PrivateState | `joy-nox-zheng-private-execution-v1` | exact trusted BBG root and authenticated public tables |

Public profiles retain their disclosure contract. Private state hides the query
witness while disclosing its authenticated public tables. The profile's state
presence is checked independently even where two profiles share an artifact
format. No profile fallback or legacy statement acceptance exists.

## Verification

`verify_result(&ExpectedJob, ProducedProof)` checks the saved job ID, attempt,
profile/header and artifact byte limit before decoding. It invokes the selected
native artifact verifier, then compares the verified coordinates with the saved
program, public inputs, optional expected output, budget and state root.
The assembly and program name must match the exact saved bundle; state ABI and
source identity are checked where the selected artifact authenticates them.
A public stateless artifact authenticates machine execution, not source
compilation or the bundle's descriptive/cost metadata. Its program name is
checked for consistency and remains unauthenticated metadata. No profile proves that
the compiler preserved source semantics.

At most 64 public input words, 4096 expected output words and 256 MiB of artifact
bytes are admitted by this adapter; each selected decoder/proof profile may
impose tighter limits. Budget and byte limits must be nonzero. The authenticated
statement budget and actual reductions must both be within the saved budget.
Unsupported target OS names fail; nox and its cyber environment are accepted.

On success `VerifiedExecution` supplies read-only job/attempt/profile, program
particle, verified public input/output, reductions, statement budget and state
root. The program particle follows Joy's existing Hemera-of-trimmed-assembly
identity. Workers cannot supply a separate authoritative output or cost field.
Errors are typed and use static safe messages without witness or artifact data.

## Caller responsibilities

The scheduler must load the expected job independently of the result. Rebuilding
expectations from worker-supplied fields defeats this boundary. An optional
expected output means the proof determines the result; providing one additionally
checks the caller's expected computation outcome.

A successful verification is a checked computation, not persistent acceptance,
network finality or reward eligibility. The job ID/attempt comparison establishes
association with saved dispatch state; it is not a proof of freshness. Current
artifacts do not bind network identity, job nonce or height into their statement.
The same valid computation may legitimately satisfy multiple matching local
jobs. Cryptographic freshness needs an owner-defined statement commitment.

Cyber must separately enforce cancellation, terminal attempts, trusted-root
freshness and atomic durable acceptance before an accepted result authorizes any
state effect. This in-process verification adapter makes no hard wall-clock,
memory-isolation or cancellation guarantee. The existing signed graph-action
endpoint and its receipts retain their own meaning; no worker endpoint is added
or implied by this library.
