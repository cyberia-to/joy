# Soft3-only Joy — historical delivery

Superseded for private proving by [native-private-parity.md](native-private-parity.md).
The refusal below describes the earlier removal commit, not current behavior.

User-directed correction: remove Joy-owned foreign private prover integration,
all foreign Cargo dependencies/patches and current capability claims. Keep
native nox execution, compiler jobs and public Zheng/state certificates.
Reject secret/--zk proving before any output write; old JOYZK envelopes cannot
fall back to execution or legacy acceptance. Preserve secret execution tests.
Check embedded compiler resources separately, build with no Trisha sibling,
verify metadata closure under all features, run owner tests and installed
build/run/prove/verify + compiler-job acceptance, measure the release binary.
Deliver feature PR(s) to release/0.4; default branches remain untouched.

Implemented in Joy06aac01 / Trident7b1d4c0 / Trishaaa25e32. Source gates:
124 Joy;724+724 Trident library,26 import/target,14+13 differential;430 Trisha
with6 existing ignored,133/43 baseline rows unchanged.65/21 installed generated
compiler acceptance retains all204 files and exact C1. Installed Joy4571120B,
stripped4001360B, native87 SDK modules retained. See audit/soft3-only/.
Delivery branches: Joyfix/0.4-soft3-only, Tridentfix/0.4-soft3-resources,
Trishafix/0.4-explicit-external-compiler; integrate through release/0.4 PRs.
