# Joy — Claude Code Instructions

nox VM warrior for the cyber battlefield. Execute, prove, verify,
deploy Trident programs compiled to nox formulas (the soft3 stack).
Built the way trisha is built; trident's CLAUDE.md rules (honesty,
forbidden patterns, review passes, git workflow) apply here too.

## Workspace

This repo (`~/cyber/joy`) is a Cargo workspace, companion to
`~/cyber/trident` (the compiler) and sibling of `~/cyber/trisha`
(the Triton warrior it copies in shape).

Members:
- `cli/` — binary crate `joy` (the actual `joy` command)
- `rs/`  — lib crate `joy-rs` (Warrior: Runner/Prover/Verifier/Deployer
  over cyber-nox)

Each crate uses `[lib] path = "lib.rs"` or `[[bin]] path = "main.rs"` —
**no `src/` subdirectory anywhere** (trisha convention).

```
cli/            Binary crate (package name: joy)
  main.rs       Entry point + Cli/Command + load_bundle/make_input
  error.rs      JoyError enum
  compile.rs    .tri source -> ProgramBundle via trident API
  run.rs        joy run
  verify.rs     joy verify (--proof zheng / --claim re-execution)
  prove.rs      joy prove (zheng artifact)

rs/             CPU backend (crate: joy-rs)
  lib.rs        pub mod formula/target/warrior
  formula.rs    bracket text <-> nox Reduction arena; subject builder
  target.rs     nox TerrainConfig (resolve or built-in mirror)
  warrior.rs    Warrior + SecretProvider over nox::reduce; prove/verify
  proof.rs      ProofArtifact (zheng wire form) + program_hash
  tests/
    integration.rs   end-to-end bundle -> output tests
    fixtures/        add.tri + hand-built add.bundle.json
```

## Companion repos

- **trident** (`~/cyber/trident`) — the compiler. ProgramBundle is the
  boundary (`trident/src/runtime/artifact.rs`). joy depends on it via
  `path = "../trident"`. Do NOT edit trident from this repo; trident-side
  wiring requests go to `.claude/trident-wiring.md`.
- **nox** (`~/cyber/nox/rs`) — cyber-nox, the execution engine:
  `reduce()`, `Tracer`, `Reduction` arena, `CallProvider`.
- **trisha** (`~/cyber/trisha`) — the Triton warrior; joy copies its
  CLI shape and error style.
- Use repo-qualified paths when referencing across repos
  (e.g. `joy/rs/warrior.rs` vs `trident/src/cli/mod.rs`).

## What works vs what's a dash (M4 of the soft3 release)

**Working**: `run` (nox reduce with VecTrace; public inputs = subject
cons list, secret inputs = call-pattern witnesses), `verify` by
re-execution, `.tri`/`.json`/`.nox` inputs, `prove` (zheng proof ->
`<name>.zheng.json`), `verify --proof` (zheng verification, no
re-execution).

**Dash**: deploy (post-M4), bbg look proving (M6 — prove refuses
tag-17 traces), hash-block proving (prove refuses tag-15 traces until
HashAux wiring). The dash is the release note — never fake a proof,
never print a number the system didn't produce.

## Execution model

- `bundle.assembly` holds the formula in bracket notation
  (`[5 [[1 3] [1 5]]]`) — exactly what `trident build --target nox` emits.
- Public inputs bind as the subject: `[p_last [... [p_first 0]]]`
  (mirrors NoxCompiler's `Scope::bind`).
- Secret inputs are served sequentially by `SecretProvider` to call
  patterns (tag 16).
- The `Reduction` arena is stack-allocated; execution runs on a
  dedicated 256 MiB worker thread. Never call `reduce` on the main
  thread with a big arena.
- Reduction count = trace rows = budget consumed (one row per unit,
  by nox design).

## CLI contract

```
joy run    <bundle.json | file.tri | file.nox> [--input-values 1,2] [--secret 3] [--budget N]
joy prove  <input> [--input-values ...] [--secret ...] [--output p.zheng.json]
joy verify <input> --proof <p.zheng.json>            # zheng proof, no re-execution
joy verify <input> --claim <values> [--input-values ...]  # re-execution
```

stdout = machine-readable output, stderr = progress/diagnostics.
`--target` accepts `nox` (terrain) or `cyber` (battlefield); anything
else is refused with a pointer to trisha.

## Forbidden patterns

Same as trisha: no `HashMap` (use `BTreeMap`), no `.unwrap()` outside
tests, no `println!` in library code, no floating point, no file > 500
lines.

## Build & Test

```
cargo check --workspace --all-targets   # zero warnings
cargo test
cargo install --path cli --force        # installs `joy` on PATH
```

## Git Workflow

Atomic commits. Conventional prefixes: `feat:`, `fix:`, `refactor:`,
`test:`, `docs:`, `chore:`. Rebuild after commit:
`cargo install --path cli --force`.

## License

Cyber License: Don't trust. Don't fear. Don't beg.
