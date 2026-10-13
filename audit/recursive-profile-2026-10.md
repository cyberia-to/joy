# recursive profile in `joy prove --succinct` — 2026-10-09

Contract: `specs/cli.md` (`joy prove --succinct [--machine]`); zheng
envelope profile 5 (zheng `specs/api.md` § recursive, PR cyberia-to/zheng#54).

Stand: joy `feat/ivc` on `feat/accumulation` 639bcea; zheng
`feat/ivc-envelope` 7e52f55 (on `feat/ivc`, #53). Apple M4 Max, `--release`,
**machine shared with other agents (load average 23–42)**: single runs,
upper bounds of a quiet machine. Sizes are deterministic.

## selection

| program | `joy prove --succinct` | `--succinct --machine` |
|---|---|---|
| fits the one-relation compiler | profile 1 (succinct) | profile 1 |
| beyond it (nesting > 128, > 4096 nodes, too many rows) | **profile 5 (recursive)** | profile 4 (machine) |

`joy verify` accepts profiles 1, 4 and 5 by their header.

## measurements

joy CLI (`joy prove` wall time as printed; `joy verify` = process wall
time, `/usr/bin/time -p`, 3 runs):

| program | profile | envelope B | prove | verify (whole process) |
|---|---|---|---|---|
| hash.tri (7), 61 reductions | 1 | 16,308 | 19.5 s | 0.01 s |
| loop.tri (140 squarings, 3), 3,245 reductions | 5 | 297,613 | 87.8 s | 0.27–0.38 s |
| loop.tri, same | 4 (`--machine`) | 147,743 | 86.9 s | 0.01 s |

zheng library, the same fixtures for every profile (zheng
`audit/recursive-envelope-2026-10.md`; verify = decode + verify, median of
5, key cached):

| fixture | profile | envelope B | prove s | verify ms |
|---|---|---|---|---|
| hash.tri | 1 | 15,319 | 6.0 | 7.81 |
| hash.tri | 4 | 97,212 | 38.0 | 6.00 |
| hash.tri | 5 | 284,950 | 47.7 | 26.50 |
| merkle-32 | 1 | — (relation compiler: `Limit`) | — | — |
| merkle-32 | 4 | 222,704 | 133.4 | 13.00 |
| merkle-32 | 5 | 285,303 | 80.5 | 32.12 |

The profile-5 CLI verify is dominated by deriving the recursion circuit
key, once per process (223–315 ms, zheng `ivc_bench` with `ZHENG_DRY=1`);
a long-lived verifier pays it once.

## decision

Recursion does not dominate the machine: for runs of one segment the
machine proof is half the size (148 vs 298 KB on loop.tri) and verifies
in milliseconds without a key derivation. It does bound every run: the
recursive proof is 282–298 KB from 33 cycles to 1.57M (zheng
`audit/recursion-2026-10.md` §3), while the machine proof grows ~96 KB
per 2^14-row segment (zheng `audit/accumulation-2026-10.md`) and passes
the recursive size near three segments; proving time is comparable
(merkle-32: 80 s recursive against 133 s machine). So:

- default beyond one relation: **profile 5**, the only profile whose
  size and verify time do not depend on the run;
- profile 4 stays reachable with `--succinct --machine` for short runs
  where its smaller proof matters.

Neither meets the ≤ 64 KB / ≤ 1 ms goal; the route is zheng's wrap step.
Open: persist the deterministic circuit key so a fresh `joy verify` does
not re-derive it.
