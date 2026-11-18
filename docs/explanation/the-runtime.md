# the runtime: one command, four actors

joy is the warrior of the ladder: it runs a program, proves that it ran, and reads the state it ran against. one command does all three. this page explains what happens behind it, who the four actors are, and what a reader may trust at the end, with the numbers that exist today and the ones that do not yet.

```
joy prove program.tri --state state.json --input-values 3,5 --output program.zheng
joy verify program.zheng --state state.json --claim 24 --input-values 3,5
```

```
program.tri ──trident──▶ nox trace ──zheng──▶ program.zheng (the certificate)
                              ▲                    ▲
            state.json ──bbg──┘ reads        lens ─┘ every commitment and opening
```

the four actors: **bbg** is the book the program reads, **lens** is the seal on every page, **zheng** is the witness of every step, **joy** is the clerk who takes the case and hands back one file. the arithmetic under all of them is one field (Goldilocks, [nebu](https://github.com/cyberia-to/nebu)) and one hash ([hemera](https://github.com/cyberia-to/hemera)); nothing else is trusted.

## the book: bbg

[bbg](https://github.com/cyberia-to/bbg) is the authenticated state of cyber. what a program can read is not a database it has to believe: it is a book whose cover carries thirteen seals, each a 32-byte hemera hash, 416 bytes in all ([bbg/specs/state.md](https://github.com/cyberia-to/bbg/blob/master/specs/state.md) § BBG root). nine seals are public summaries: particles, axons by source and by target, neurons, locations, coins, cards, files, time. three are private: the cyberlinks themselves, the spent set and the balance bitmap, so who linked what is never disclosed while the aggregate is. one closes the finalized signals.

joy does not sync a live network. `--state state.json` loads a bounded, authenticated certificate of that book: the roots and the pages the program may touch. every actual read the program makes is bound into the proof as four facts: the namespace, the index, the value returned, and the root it hangs from (all four limbs of it). a verifier who holds the same `state.json` therefore knows not only that the program computed correctly but that it computed over this state and no other. the scheme is `joy-nox-public-state-execution-v1` ([README](../../README.md)); its bounds today: all ten public namespaces present, at most 2048 fields in total.

bbg's three laws are what make this cheap enough to be a habit: a local change costs what it touches, never a global recompute; verifying a proof costs the same whatever the computation cost; and a lie about the structure is caught by the structure, not by the honesty of a protocol ([bbg/specs/architecture.md](https://github.com/cyberia-to/bbg/blob/master/specs/architecture.md)).

## the seal: lens

[lens](https://github.com/cyberia-to/lens) is a polynomial commitment. picture a table of numbers; the mathematics calls it a multilinear polynomial in ν variables, with 2^ν cells. lens seals the whole table into 32 bytes and can later prove the number in any one cell without showing the table.

sealing: the table is first encoded with redundancy, so that changing one cell afterwards would show in a large share of positions, then hashed with hemera. the 32 bytes are the commitment. opening: the verifier names a point; the prover answers with a value and a proof built in rounds, each round halving the table along one coordinate of the point; in every round the verifier asks for a handful of positions of the encoded table and checks them against the seal. which positions, the prover cannot know in advance: they are drawn from the hemera hash of everything said so far (Fiat–Shamir), which is what turns a conversation into one document anyone can check later. the seal binds (the table cannot change after the 32 bytes are out) and the opening is sound (a false value needs a hemera collision); there is no trusted setup, no pairings, and the whole thing is post-quantum ([lens/specs/commitment.md](https://github.com/cyberia-to/lens/blob/master/specs/commitment.md)).

lens carries five constructions over five algebras. zheng opens its tables with the Reed–Solomon one, `rspcs`, by WHIR over hemera Merkle trees; Brakedown, over a Margulis expander, is the primary construction for the other algebras.

## the witness: zheng

[zheng](https://github.com/cyberia-to/zheng) proves that the nox machine ran the program. every step of the machine becomes one row of a large table; the rules of the machine become constraints every row must satisfy (a CCS, checked by Spartan with sumcheck). zheng does not show the table: it seals it with lens and answers the verifier's random questions. the certificate also pins the statement itself: the verifier recompiles the relation from the program and fixes the inputs, the outputs and the step count by its own hand, so the proof cannot be moved to another program or another claim.

proofs come in profiles, and the honest reading of them is the table below, measured on code revision a076137 (Apple M4 Max, 16 threads, single-machine medians; the receipt with load, commands and envelope hashes is [zheng/audit/consistency-2026-10-27.md](https://github.com/cyberia-to/zheng/blob/master/audit/consistency-2026-10-27.md)):

| profile | what the verifier checks | example | envelope | prove | verify |
|---|---|---|---|---|---|
| 0 · public certificate | every row of the disclosed witness, exactly; linear size | | | | |
| 1 · succinct | Spartan over Fp3, then one WHIR opening | hash.tri, 2^10 variables | 16,791 B | 39.6 ms CPU · 19.2 ms Metal | 4.9 ms |
| 1 · succinct | | Merkle membership, depth 32 (2^18) | 63,594 B | 1,347 ms CPU · 827 ms Metal | 324 ms |
| 2 · zk | masked Spartan and a hiding tensor opening; statistical zero knowledge | hash of a secret preimage | 63.9 KB (an earlier measurement, [zheng/audit/zk-profile-2026-10.md](https://github.com/cyberia-to/zheng/blob/master/audit/zk-profile-2026-10.md)) | | |
| 3 · state-public | profile 0 plus every read authenticated under the state root | | | | |
| 4 · machine | the nox machine as one step relation, accumulated per segment | hash.tri | 96,083 B | 7.4 s CPU | 5.0 ms |
| 5 · recursive | each step verifies the previous one; constant in the number of steps | hash.tri | 407,947 B | 10.5 s CPU | 9.5 ms |
| 6 · wrapped | profile 5's verifier, proved by three wrap levels | hash.tri | 62,687 B | 66.8 s CPU, 12.1 GB peak | 5.4 ms |

the goal the owner set on 2026-10-09: any nox computation in at most 64 KB, post-quantum, verified in at most 1 ms, constant in the number of steps. profile 6 meets the size and the constancy; **no profile meets 1 ms yet**. profile 1 runs at 128 proven bits with grinding, 106–112 without. the rows of the soundness ledger that are conjectured rather than proven (the relation compiler, the hash, program binding by digest, the recursive Fiat–Shamir of IVC and wrap) are listed in [zheng/specs/soundness.md](https://github.com/cyberia-to/zheng/blob/master/specs/soundness.md).

## the clerk: joy

three verbs. `joy run` executes the program on nox and prints the result. `joy prove` executes it and writes the certificate: a self-contained artifact that carries the program, the inputs, the outputs, the reduction count and, with `--state`, the root of the book. `joy verify` checks a certificate without running nox; passing the program source beside `--proof` also checks that the certificate belongs to that program. public inputs bind as `[p_last [... [p_first 0]]]`.

what the keys select: without keys, `zheng-nox-public-execution-v2`, a public certificate that discloses the witness and verifies in linear time. `--zk`, or `--secret <value>` which implies it, selects `joy-nox-ccs-triton7-zk-v3`: trisha supplies a randomized STARK over the exact zheng CCS, private columns are absent from the artifact, and the verifier learns that everything holds without learning what. `--state` with `--zk` hides the coordinates of the reads as well.

what joy does not do today: dynamic continuations and variable branch shapes are unsupported; budgets must cover the authenticated cost of the selected path; the relation bounds every cost below the field modulus. the capabilities file that states this is [targets/nox/capabilities.json](../../targets/nox/capabilities.json).

## what you may trust, and why

- the seal binds and the witness is sound: both reduce to collisions in hemera. no trusted setup, no pairings, post-quantum.
- the certificate is tied to this program, these inputs, these outputs and this state root; moving it elsewhere breaks the statement the verifier recompiles.
- the verifier never runs the program and never sees the book; with `--zk` it never sees the secrets either.
- the limits are real: 1 ms verification is not met; several soundness rows are conjectured; the sizes above are medians on one machine; joy reads a state certificate, not a live chain.

## try it in five minutes

```sh
cargo install --git https://github.com/cyberia-to/joy joy   # the crate named joy on crates.io is somebody else's; install from the repository
joy describe --target nox
joy run program.tri --input-values 3,5
joy prove program.tri --input-values 3,5 --output program.zheng
joy verify program.zheng --claim 24 --input-values 3,5
```

the old `zheng-hypernova-tensor-merkle-v2` artifacts of the 0.3 fold establish nothing; the fold was unsound and compiles only behind a feature flag. a certificate is worth what its scheme name says, and the names above are the ones that hold.
