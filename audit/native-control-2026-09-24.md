# Native source control acceptance — 2026-09-24

Joy source/test revision: `ecc17f4` on `feat/0.4-native-control-acceptance`.
Compiler revision: Trident `4317c767607ce83210348d285c1390df0bed5d94`.
Nox runtime: `5271961a72a1f1e9922df36e3e4e3c65d1acff81`.

`CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked`
passed 89 tests, zero failed/ignored. The workspace/all-target check is clean.
Logs: `/tmp/joy-04-control-committed-tests.log`,
`/tmp/joy-04-control-check.log`; their hashes and full owner revisions are in
[Trident's shared validation manifest](https://github.com/cyberia-to/trident/blob/release/0.4/audit/self-hosting/sh1-control-validation.json).

The new CLI regression compiles a source function calling an increment helper
5000 times, executes the complete ART1 and independently reads atom5000 from the
canonical output. Its program stays below 30000 bytes. The default frame cap
rejects the run; an explicit 65536-frame allowance succeeds. Frame, budget and
node failures preserve the previous output exactly, even with `--force`.

Actual installed-CLI observations also execute constant loops 0/1/4097/5000 and
one dynamic artifact with those four inputs. Source texts, complete receipts,
binary hash and output comparisons are in
[the shared observation file](https://github.com/cyberia-to/trident/blob/release/0.4/audit/self-hosting/native-control-observations.json).
All use NoTrace. This closes reusable raw source execution acceptance;
production compiler JOB1/RES1 admission and native Zheng compiler proofs remain
separate gates. No release version, default branch or runtime limit was changed.
