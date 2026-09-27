# Native private execution review

Source review and focused integration acceptance for the native-private worktrees.
Joy base: `1f9f06e44f991de96fb955a1871b9ee7797dd5d6`; Zheng base:
`80a8ca90208c3b49c4f2a8507d75fd17785fdfe3`. The implementation was uncommitted
when reviewed and tested. The hashes below identify the reviewed changes;
these observations are local evidence, not a release-candidate receipt.

The review covered:

- `joy/rs/zk_execution.rs`, `joy/rs/zk_state.rs`, `joy/rs/private_wire.rs`,
  `joy/rs/traits.rs`: statement/build identity, expected claims, authenticated
  state tables, native budget, profile dispatch and artifact admission.
- `zheng/rs/src/execution/zk/{mod,circuit,views,wire,tests}.rs`: every CCS
  residual, constant/public coordinates, additive sharing, multiplication
  replay, commitments, transcript, entropy and bounded canonical decoding.
- `zheng/rs/src/execution/{private,private_state,relation,relation_eval,
  relation_look,statement}.rs`: call checks, activity and stream order, public
  bindings, hidden cell selection and all root limbs.
- `bbg/rs/src/certificate.rs`: table authentication before relation derivation.
- `strata/nebu/rs/field.rs`: representation supporting the documented field
  buffer zeroization cast.

No remaining acceptance bypass or witness disclosure was found in that source
review. The verifier reconstructs the relation and checks the complete committed
first message before accepting its Fiat–Shamir challenges. Only the selected pair
of views is opened. Public values are circuit residuals under the same protocol.
This is an implementation review, not an independent cryptographic audit.

Review findings repaired before acceptance:

- Repeated sparse factors now pass the expanded-work bound before allocation.
- The proof-specific serde visitor caps initial reserve instead of trusting an
  admitted but truncated large length hint.
- The prover retains commitments between passes and regenerates output shares,
  avoiding retention of every round's output vectors.
- Stateless proving now executes with its explicit budget instead of the
  warrior's separately configured runtime budget.

The protocol uses the arithmetic decomposition in
[ZKBoo, sections 4.1.1–4.2 and Appendix A](https://www.usenix.org/system/files/conference/usenixsecurity16/sec16_paper_giacomelli.pdf).
Its security depends on fresh operating-system entropy, Hemera binding/hiding
and pseudorandom expansion, and the Fiat–Shamir random-oracle assumption.
The repetition bound is an interactive soundness parameter; it does not confer
an independently established production or post-quantum security level.
Proof size is linear in the admitted circuit. Public program, inputs, outputs
and selected cost remain visible; state tables remain public. Application trust
in a particular state root still requires an expected-root comparison. Binding
a source identity does not itself prove compilation correctness.

Existing bounded-relation limits remain: dynamic continuations, different branch
tree shapes and arbitrary noun call witnesses are not covered by this profile.
Zeroization is best-effort: caller-owned witnesses, old allocation copies and
hash-stream state are outside the view-buffer wipe guarantee.

Focused acceptance command, run from the Joy worktree with the bases and source
snapshot recorded here:

```sh
CARGO_TARGET_DIR=/Users/master/cyber/.worktrees/selfhost-0.4-native-private/target cargo test -p joy-rs --test native_private --release --locked --offline
```

Result: **5 passed, 0 failed, 0 ignored**, with no compiler warnings.
Log: `/tmp/joy-native-private-wrapper-review.log`. Tests reuse a valid proof
across statement, build identity and wire mutations; check independent proof
randomness, public/private trait selection, claim substitution, exact budget
override, stream mismatches, bounded outer metadata/lengths, inner truncation,
trailing/noncanonical data, hidden namespace selection and certificate metadata.
No privacy conclusion is inferred from a raw-byte search. Backend simulator and
adversarial tests are separately maintained in Zheng's reviewed test module.

Source snapshot command: `shasum -a 256` followed by these paths, from Joy:

| Path | SHA-256 |
|---|---|
| `rs/tests/native_private.rs` | `b8cde183cdceb75f60ab55f124f2d86c97aee183247a9749fad200b27542654d` |
| `rs/zk_execution.rs` | `cecc252c9a72affae53352866ebab0b24bfd7af1269f936337cea9b8b992db78` |
| `rs/zk_state.rs` | `af63fe21a893830ae0415e33a06087c7da463ead12e769c0cf233c37b33db590` |
| `rs/private_wire.rs` | `209b31749d1942acd2ed27cdc0452c5e75427215b78a3d2979b168b6ef54a0f7` |
| `rs/traits.rs` | `2f07223aff94c68d42f3e94db85c4c92b2bb9b950f3f1d1d012bf047ce054cc1` |
| `../zheng/rs/src/execution/zk/mod.rs` | `e2bbb847b57adee8dac1c4c82c355adc5bffc6b0d8eb07bd2a0e0acb9b59d761` |
| `../zheng/rs/src/execution/zk/circuit.rs` | `56d971472f6a33d5199bb2e3bacca1362cead62c9a8f204c328097745fa143d2` |
| `../zheng/rs/src/execution/zk/views.rs` | `cf1cb71a5e0f41d691b888ccbeb8f8d1b02ff6f4c4c6cd2f517498959cd65c68` |
| `../zheng/rs/src/execution/zk/wire.rs` | `1b6b2d59c0e7016f7430a6c949c6bcc5279022d45fb0e0ba73046205d7f95aff` |
