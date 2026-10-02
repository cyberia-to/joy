# Structured native certificate integration

This delivery adds public ART1/JOB1/RES1 execution certificates to Joy through
`prove-artifact` and `verify-artifact`. Zheng validates bounded derivations;
verification never invokes the Nox evaluator or a compiler. Complete expected
program/input particles, result, charged reductions and expanded logical work
are bound. Physical allocation, collection and elapsed-time observations remain
explicitly unattested. Disclosure and administrative bounds are normative in
`specs/structured-certificates.md`.

## Source and commands

Base Joy revision: `bd91dd6281714ea794672ba83c26bf7bf12d1bad`. `source.json` identifies every tested
source byte and exact clean sibling revision. `final-source.json` identifies
the committed delivery bytes. `post-gate-docs.json` records only the output
contract wording clarified after tests; Rust source is identical. The plan
status was updated after the gates. No test result is attributed to a later
implementation.

`receipt.json` retains the actual Rust 1.89.0 native host version, exact argv,
exit status, duration and raw-log SHA-256 for these commands:

- `cargo test --workspace --release --locked --offline`: 215 passed,
  0 failed, 1 existing ignored census diagnostic;
  `gate-1.log` is the complete test output.
- `cargo check --workspace --all-targets --all-features --locked --offline`:
  success, zero warnings (`gate-2.log`).
- `python3 scripts/check-soft3-boundary.py`: success (`gate-3.log`).
- Explicit Rust 1.89 rustfmt check on all listed changed Rust sources:
  success (`gate-4.log`).

Cargo and rustfmt are selected directly through rustup, and RUSTC selects the
actual 1.89 compiler. Build cache lives outside this source tree. The separate
`focused-final-rust189.json` records the capture author's actual Cargo/rustc
versions, exact environment, source hashes and command for nine capture tests.

## Review and acceptance

`integration-review.json` binds the root integration review and independent CLI
review to exact source hashes. `../structured-certificates-review/` retains the
independent verifier review, hand-authored valid derivations, rebuilt-chain
semantic attacks, all exploratory failures and the final passing commands.
Fresh-process CLI tests cover complete raw nouns, fixture compiler responses,
atomic publication, destination preservation and regular-file admission.

The compiler response fixtures deliberately construct containers and do not
stand in for the actual compiler. Frozen C2 pilot certificates, six-platform
profile checks and both full self-build certificates require their own retained
receipts. This component delivery closes no SH7/SH8 acceptance milestone by
itself and supplies no new release-distribution acceptance.
