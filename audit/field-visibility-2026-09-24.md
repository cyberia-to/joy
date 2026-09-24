# Explicit fixture field visibility

Joy source `612cfa1373ad2070dc105a39a59cc4633cd6a8b0`, tested against
Trident `2c66db3cb133ee6b461b7dc9f6e3cb0662a59c54`.

The imported-loop regression fixture intentionally reads `Pair.a/b` from its
entry module. These fields now declare `pub`, matching Trident's enforced
module privacy. Execution and certificate assertions are unchanged.

`CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked`:
89 passed, no failures or ignored tests.
`CARGO_TARGET_DIR=../trident/target cargo check --workspace --all-targets --locked`:
pass without Rust warnings. Commands ran before the fixture commit; tested
source was committed unchanged. The initial private-field rejection is retained
in `/tmp/joy-04-privacy-tests-before-public-api.log`.

The coordinated receipt with dependency pins and log digests is
`trident/audit/self-hosting/sh1-privacy-validation.json`. This is local
development compatibility evidence for `release/0.4`.
