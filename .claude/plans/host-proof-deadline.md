# Explicit compacting host deadline

Prepare a feature PR into release/0.4. The original Linux whole-proof runs
remain failed at the previous two-hour host ceiling; their sources and evidence
stay immutable. A later whole-proof run requires a separate reviewed profile
and committed Joy pin. Active local proofs, corpora and distribution remain
bound to their original pins.

1. Extend only the explicitly selected compacting host deadline to four hours.
   Preserve the 30-second default, ordinary five-minute ceiling and every work,
   storage, JOB1 and certificate-context limit.
2. Check boundaries, output/cost equivalence, identical certificate bytes across
   deadlines, fresh cross-deadline verification and failed-publication safety.
   Run the workspace release tests, all-target check and soft3 boundary gate
   with the actual Rust1.89 toolchain; retain raw evidence under audit/.
3. Review independently, commit and push, rebuild the installed CLI, then open
   the feature PR. Linux phase splitting and pending artifact transport belong
   to the separate Trisha CI branch. Neither source changes nor passing small
   tests establish a completed Linux whole proof.
