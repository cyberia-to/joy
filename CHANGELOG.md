# Changelog

## 0.1.0 — 2026-09-07

M3 of the soft3 release: joy is born.

- `joy run` — execute ProgramBundles (or .tri sources, or raw .nox
  formulas) on the nox VM via `reduce()` with a trace; public inputs
  bind as the subject, secret inputs feed call patterns.
- `joy verify --claim` — verification by re-execution.
- `joy prove` — honest dash: exits 1 until the zheng prover (M4).
- Warrior implements trident's Runner/Prover/Verifier/Deployer traits;
  the non-Runner paths answer honestly with their milestone.
