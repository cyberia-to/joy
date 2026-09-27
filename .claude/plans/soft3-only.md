# Soft3-only Joy

User-directed correction: remove Joy-owned foreign private prover integration,
all foreign Cargo dependencies/patches and current capability claims. Keep
native nox execution, compiler jobs and public Zheng/state certificates.
Reject secret/--zk proving before any output write; old JOYZK envelopes cannot
fall back to execution or legacy acceptance. Preserve secret execution tests.
Check embedded compiler resources separately, build with no Trisha sibling,
verify metadata closure under all features, run owner tests and installed
build/run/prove/verify + compiler-job acceptance, measure the release binary.
Deliver feature PR(s) to release/0.4; default branches remain untouched.
