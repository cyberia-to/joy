# Changelog

## 0.2.2 — 2026-09-08

- artifacts are compact binary (`<name>.zheng`, postcard) instead of JSON:
  add.tri 19.6 KB → 5.4 KB, two secrets 74 KB → 20 KB, a hash 189 KB →
  52 KB. 0.2.1 JSON artifacts still load.
- the prover's folded witness is no longer on the wire (zheng 0.2.1
  `serde(skip)`): it was the prover's private folded state — for
  divine() programs, the secrets' image — and the verifier never read it.
- what remains is ~1.7 KiB per accumulator group, one group per CCS
  structure (3 / 12 / 23 above); collapsing groups toward a 2–5 KB
  program-level proof is zheng#8.

## 0.2.0 — 2026-09-08

- hash blocks prove: HashAux (sponge rate = cached structural digest of
  the hashed input) built inside the traced worker; hash.tri fixture
  (trident hash builtin) proves in ~660 ms debug, ~189 KB artifact,
  verifies in ~140 ms; tampered artifacts reject.
- look proving against a real BbgState via
  `Warrior::prove_zheng_with_state`: looks answer from the state and
  record Brakedown openings, the statement carries `state.root()` as
  the public root. Cross-repo e2e lives here (joy sits above zheng and
  bbg): hand-built .nox look program (trident cannot express a state
  read yet — follow-up noted), prove -> verify PASS; stale root refuses
  at prove; re-rooted and group-tampered artifacts reject.
- CLI `--state` on prove: honest error — bbg has no whole-state file
  format yet; the library path is wired.
- `joy prove` — execute via nox with the tracer, fold the trace with
  zheng (SuperSpartan + Brakedown + HyperNova accumulators), write a
  `<name>.zheng` artifact: statement + proof + metadata. Statement
  binds the program (hemera of the assembly), the first/last trace rows
  (input/output hashes), the budget (focus bound); bbg_root is the
  zero sentinel (stateless programs).
- `joy verify --proof` — zheng verification, no re-execution. Rejects
  proofs for a different bundle, tampered proofs, malformed artifacts,
  foreign formats. Output names the mode: "PASS (zheng proof)" vs
  "PASS (re-execution)".
- add.tri fixture chain measured (debug build): prove ~285 ms for
  10 reductions, artifact ~44 KB, verify ~90 ms.
- `reads_state` follows trident's additive ProgramBundle field
  (compat fix; joy does not yet cons the live root per the flag).

## 0.1.0 — 2026-09-07

M3 of the soft3 release: joy is born.

- `joy run` — execute ProgramBundles (or .tri sources, or raw .nox
  formulas) on the nox VM via `reduce()` with a trace; public inputs
  bind as the subject, secret inputs feed call patterns.
- `joy verify --claim` — verification by re-execution.
- `joy prove` — honest dash: exits 1 until the zheng prover (M4).
- Warrior implements trident's Runner/Prover/Verifier/Deployer traits;
  the non-Runner paths answer honestly with their milestone.
