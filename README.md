# Joy

nox warrior — the cyber battlefield. Executes and verifies Trident
programs on the nox VM, the proof-native machine of the soft3 stack.

```
source .tri → trident → ProgramBundle (.nox formula) → joy
                                                        ├── run     nox reduce, traced
                                                        ├── prove   zheng proof -> .zheng
                                                        └── verify  the proof — no re-execution
```

Trident is the weapon; warriors wield it. Trisha fights on Triton,
joy fights on nox: terrain nox, battlefield cyber.

## what it does

```
joy run    program.tri --input-values 3,5       # compile via trident, execute, print output
joy run    bundle.json                           # execute a compiled bundle
joy run    formula.nox --secret 42               # execute a raw nox formula
joy prove  program.tri --input-values 3,5        # execute + zheng proof -> program.zheng
joy verify program.zheng                   # self-contained artifact, NO re-execution
joy verify program.tri --proof program.zheng  # same, bound to this bundle
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
| prove (zheng) | works — `<name>.zheng` artifact |
| verify a zheng proof | works — no re-execution |
| deploy (particle + cyberlinks) | — after M4 |
| bbg look (pattern 17) | works via joy-rs API (`prove_zheng_with_state`, public root in the statement); CLI `--state` awaits a bbg state-file format |
| hash blocks in proofs | works — HashAux built from the arena's cached digests |

The dashes are the release notes. No gates, no fakes.

## proof artifact

`joy prove` writes `<name>.zheng` next to the input (or `--output`):
statement (program/input/output hemera hashes, focus bound, bbg root
sentinel) + the zheng trace proof + metadata. `joy verify --proof`
recomputes the program hash from the bundle, checks it against the
statement, and runs `zheng::verify`. The executed outputs in `meta` are
unverified: `statement.output_hash` hashes the final trace row, whose
result is an arena identifier rather than the flattened output values.

## build

```
cargo install --path cli --force     # or: cargo install cyber-joy
```

Depends on sibling repos by path: `../trident` (compiler,
ProgramBundle), `../nox/rs` (cyber-nox, the VM), `../strata/nebu/rs`
(Goldilocks), `../zheng/rs` (the prover, `serde` feature for the
artifact wire form), `../hemera/rs` (program hashing).

## license

cyber license: don't trust. don't fear. don't beg.

### verification claims

The current proof format is `zheng-hypernova-tensor-merkle-v2`. Regenerate
older artifacts with the authenticated TensorMerkle PCS. Binary and JSON
representations use the same format identifier.

`verify <artifact>` and `verify <program> --proof <artifact>` check the trace
statement. Emitted output values and cycle counts in artifact metadata are
reported as unverified. The final-row hash authenticates arena identifiers,
not a flattened output vector. Proof verification therefore rejects `--claim`;
use `verify <program> --claim <values>` for verification by execution. For the
same reason the generic `Verifier` trait refuses `ProofData.claim` until an
output-to-arena relation is proved. `verify_artifact` and `verify_zheng` are
the explicit statement-only library APIs.
