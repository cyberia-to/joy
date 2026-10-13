# wrapped profile in `joy prove --succinct` — 2026-10-13

Contract: `specs/cli.md` (`joy prove --succinct [--machine | --wrap |
--no-wrap]`, key cache); zheng envelope profile 6 (zheng PR #57,
`specs/api.md` § wrapped, § key bundles).

Stand: joy `feat/wrap` (on `feat/ivc`, #38); zheng `feat/wrap-envelope`
(on #56 at 606cda1). Apple M4 Max, 48 GB, `--release`. **Shared machine**:
other agents ran wrap chains in parallel, load average up to 157, swap
up to 42 GB, disk 97–99 % full; times are upper bounds.

## selection

| program | `--succinct` | `--wrap` | `--no-wrap` | `--machine` |
|---|---|---|---|---|
| fits one relation | 1 | 1 | 1 | 1 |
| beyond it | 6 if available memory ≥ 32 GiB, else 5 + note | 6 | 5 | 4 |

Threshold `joy_rs::memory::WRAP_PEAK_BYTES` = 32 GiB: wrap proves reached
max RSS 27.1–29.2 GB, the chain's key derivation 30.9 GB; peak footprints
(compressed + swapped) 48.7–60.3 GB.

## measurements (joy CLI, process wall time)

| artifact | profile | bytes | prove | max RSS | verify, keys derived | verify, keys cached |
|---|---|---|---|---|---|---|
| loop.tri (140 squarings, 3), 3,245 reductions, `--no-wrap` | 5 | 337,229 | 56.7 s | 6.2 GB | 2.54 s | 0.13 s (×3) |
| loop.tri, `--wrap` | 6 | — | killed by the OS twice (945 s and 1,064 s, footprint 48.7 GB, swap and disk exhausted by concurrent runs) | 27.0–28.5 GB | — | — |
| zheng's hash.tri profile-6 envelope (`--budget 2^40`) | 6 | 62,962 | (zheng: 2,511 s, 29.2 GB) | | 6.07 s | 0.39–0.74 s (load avg 157) |

The profile-6 path through joy (prove + verify of loop.tri) did not
complete on this machine in this window; joy's profile-6 verify, the key
cache (derive, install, a corrupted file rederived and rewritten) and
rejection of a flipped byte and a wrong claim were checked on zheng's
hash.tri envelope. `cli/tests/wrapped_execution.rs` (ignored) is the
end-to-end check to run on a quiet machine with ≥ 32 GiB free.
