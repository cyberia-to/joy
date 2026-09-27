# Native Joy parity

Owner correction, 2026-09-27: Joy owns soft3 + cyber; Trisha owns Triton +
Neptune. Removing a foreign dependency must preserve the proving capability by
completing its native owner. Work stays on feature branches and integrates into
release/0.4, with default branches untouched.

Accepted starting revisions: Joy1f9f06e, Zheng80a8ca9, Trident90f42b4. All sibling
inputs are isolated committed worktrees. The native nox call implementation and
Zheng private execution/state relations already exist. PublicTensor and the
current Spartan transcript disclose witness-dependent data. The cryptographic
private backend belongs in Zheng, and its orchestration belongs in Joy.

Delivery order:

1. Implement native private CCS proof/verification in Zheng over Goldilocks and
   Hemera, using the finite-ring three-party MPC-in-the-head construction.
   Bind the complete relation, constant/public coordinates, all output shares
   and all commitments before challenges. Fixed219 repetitions; fresh OS entropy;
   explicit dimensions, canonical fields and wire limits. Independently review
   malicious-prover and privacy behavior before delivery.
2. Restore Joy stateless and authenticated private-query proving, using a fresh
   native wire format. Select it for secrets/--zk; verify without witness or rerun.
   Public proofs stay explicit and secret-free. Preserve native execution
   agreement, complete artifact identity and state-root binding.
3. Align native secret streams, add bounded witness files and batch operations.
   Exercise installed commands, malformed proofs, wrong claims and source/state
   tampering. Keep the package/feature boundary gate green.
4. Compare the remaining test/bench and cyber network/action interface against
   implemented Trisha behavior. Cyber owns signed admission and receipts; a graph
   publication must never masquerade as acceptance of a computation certificate.
5. Record proof costs, binary sizes, source revisions and actual coverage under
   audit/. Commit logical units, open feature PRs, integrate accepted pieces into
   release/0.4 and reproduce installed binaries from origin.

Open boundaries must stay explicit: existing static relation coverage, public
cycle count, public BBG dimension certificates, linear proof/verification costs,
and independent cryptographic review. Complete warrior parity is not established
by a passing one-program proof or a CLI command inventory.

Implementation status: native backend and all four Joy proof profiles are
implemented. Native witness files, exact streams, batch/test/bench and typed
saved-job verification are implemented. Runtime acceptance and measured costs
are recorded in `audit/native-private/`; default branches remain unchanged.
The worker adapter ends at verified local computation. Cyber has no execution-job
admission endpoint yet; its scheduler, durable acceptance and proof context
binding remain explicit cross-component work. Full dynamic nox/self-host proof
coverage also remains open, independently of native secret proving.
