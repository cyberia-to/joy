# Joy

nox warrior — the cyber battlefield. Executes and verifies Trident
programs on the nox VM, the proof-native machine of the soft3 stack.

```
source .tri → trident → ProgramBundle (.nox formula) → joy
                                                        ├── run     nox reduce, traced
                                                        ├── verify  re-execution
                                                        └── prove   zheng (M4) —
```

Trident is the weapon; warriors wield it. Trisha fights on Triton,
joy fights on nox: terrain nox, battlefield cyber.

## what it does

```
joy run    program.tri --input-values 3,5       # compile via trident, execute, print output
joy run    bundle.json                           # execute a compiled bundle
joy run    formula.nox --secret 42               # execute a raw nox formula
joy prove  program.tri --input-values 3,5        # execute + zheng proof -> program.zheng.json
joy verify program.tri --proof program.zheng.json  # verify the proof, NO re-execution
joy verify bundle.json --claim 8                 # verify by re-execution
```

stdout carries the output values (one per line); stderr reports
`Executed in N reductions`. The reduction count is the trace row
count — one row per budget unit, and the trace IS the zheng witness.

## input model

- **public inputs** (`--input-values`) bind as the nox subject, the
  way trident's NoxCompiler binds function parameters:
  `[p_last [... [p_first 0]]]`.
- **secret inputs** (`--secret`) are served in order to nox call
  patterns (tag 16) — the prover-side witness stream.

## status (0.1.0 · soft3 release M4)

| capability | state |
|---|---|
| run (nox reduce + Tracer) | works |
| verify by re-execution | works |
| .tri / .json / .nox inputs | works |
| prove (zheng) | works — `<name>.zheng.json` artifact |
| verify a zheng proof | works — no re-execution |
| deploy (particle + cyberlinks) | — after M4 |
| bbg look (pattern 17) | — prove refuses look traces until a bbg state + root is wired (M6 consumer side) |
| hash blocks in proofs | — prove refuses tag-15 traces until HashAux wiring |

The dashes are the release notes. No gates, no fakes.

## proof artifact

`joy prove` writes `<name>.zheng.json` next to the input (or `--output`):
statement (program/input/output hemera hashes, focus bound, bbg root
sentinel) + the zheng trace proof + metadata. `joy verify --proof`
recomputes the program hash from the bundle, checks it against the
statement, and runs `zheng::verify` — the executed outputs in `meta` are
bound in aggregate through `statement.output_hash` (the hemera hash of
the final trace row), not value-by-value.

## build

```
cargo install --path cli --force
```

Depends on sibling repos by path: `../trident` (compiler,
ProgramBundle), `../nox/rs` (cyber-nox, the VM), `../strata/nebu/rs`
(Goldilocks), `../zheng/rs` (the prover, `serde` feature for the
artifact wire form), `../hemera/rs` (program hashing).

## license

cyber license: don't trust. don't fear. don't beg.
