# Changelog

## 0.2.0 — 2026-09-07

M4 of the soft3 release: the prover is real.

- `joy prove` — execute via nox with the tracer, fold the trace with
  zheng (SuperSpartan + Brakedown + HyperNova accumulators), write a
  `<name>.zheng.json` artifact: statement + proof + metadata. Statement
  binds the program (hemera of the assembly), the first/last trace rows
  (input/output hashes), the budget (focus bound); bbg_root is the
  zero sentinel (stateless programs).
- `joy verify --proof` — zheng verification, no re-execution. Rejects
  proofs for a different bundle, tampered proofs, malformed artifacts,
  foreign formats. Output names the mode: "PASS (zheng proof)" vs
  "PASS (re-execution)".
- Honest refusals: hash-block traces (needs HashAux wiring), look
  traces (needs a bbg state + public root, M6 consumer side),
  single-row traces (no transition to fold).
- add.tri fixture chain measured (debug build): prove ~285 ms for
  10 reductions, artifact ~44 KB, verify ~90 ms.

## 0.1.0 — 2026-09-07

M3 of the soft3 release: joy is born.

- `joy run` — execute ProgramBundles (or .tri sources, or raw .nox
  formulas) on the nox VM via `reduce()` with a trace; public inputs
  bind as the subject, secret inputs feed call patterns.
- `joy verify --claim` — verification by re-execution.
- `joy prove` — honest dash: exits 1 until the zheng prover (M4).
- Warrior implements trident's Runner/Prover/Verifier/Deployer traits;
  the non-Runner paths answer honestly with their milestone.
