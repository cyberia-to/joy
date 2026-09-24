# Explicit source compiler profile

Date: 2026-09-24. Joy source `5db7fa3559382de7bc3c93e7712daf28b730e24d`;
Trident source `6b8d4e37b1dbbbc86dcb952c38f667f8277a5cea`.

`build --emit artifact --artifact-profile compiler-job` now exports a source
Noun-to-Noun program with ART1 profiles1/1. Joy admits its JOB1 and validates its
RES1 before publication. Default raw output remains byte-compatible.

The installed CLI builds `rs/tests/fixtures/compiler_transport.tri`, executes its
bound result construction, extracts ART1 and separately obtains14 from that
program. It compares complete canonical bytes and preserves existing output on
invalid input. [Exact commands and observations](compiler-profile-cli.json).

Full source revisions, gate counts, previous fixture failures and formal UNKNOWN:
[Trident receipt](../../../trident/audit/self-hosting/sh1-profile-validation.json).
[SH1 combined verdict](../../../trident/audit/self-hosting/native-compiler-profile.md).
This is local development evidence. The fixture quotes its generated program;
guest source compilation remains SH2.
