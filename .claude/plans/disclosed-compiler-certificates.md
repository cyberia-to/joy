# Disclosed native compiler certificates

Continue SH7/SH8 from accepted SH0–SH6 on isolated feature branches targeting
release/0.4. Defaults, frozen compiler inputs, accepted evidence and installed
frozen Joy binaries remain untouched. Root owns Joy, other agents own bounded
Zheng semantics, nox collection snapshots and distribution validation.

Implemented and checked: steps 1–3. Transport is merged in Joy PR25;
Zheng PR40–43 and nox PR26 provide authenticated noun memory, derivations,
bounded semantic checking and complete collection snapshots. Production Joy
gates and independent mutation/review evidence are retained in
audit/structured-certificates{,-review}/. Actual frozen-compiler pilot and
both whole self-build certificates remain pending; SH7/SH8 stay open.

1. Specify and implement bounded framed transport with exact input context,
   ordered Hemera chain, strict decompression and terminal EOF. Check malformed
   and interrupted streams independently before committing this component.
2. Connect nox v2 observation to Zheng checked activations and reusable opaque
   derivations. Full noun snapshots permit GC; stored activation facts survive
   reset. Keep producer dictionaries, active calls and verifier memory bounded.
3. Add production ART1/JOB1 admission, result binding and prove-artifact /
   verify-artifact commands. Verification checks the certificate without running
   nox or any compiler. Publish complete files atomically after success.
4. Retain actual compiler workload proofs and adversarial fresh verification;
   then prove both frozen whole self-builds for SH8. Component tests and prefix
   measurements do not close these milestones. Record commands, exact sources,
   resource measurements and remaining gates under audit/.
