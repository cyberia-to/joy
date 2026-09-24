# Joy — Claude Code Instructions

nox VM warrior for the cyber battlefield. Build, execute, prove and verify
Trident programs compiled to nox formulas (the soft3 stack). Live deployment
is unsupported.
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
cli/            Binary crate (package name: cyber-joy, binary: joy)
  main.rs       Entry point + Cli/Command + load_bundle/make_input
  error.rs      JoyError enum
  compile.rs    Shared source/project/target/profile resolution via Trident API
  build_cmd.rs  Atomic bundle/nox export and build JSON-v1 results
  run.rs        joy run
  verify.rs     joy verify (--proof zheng / --claim re-execution)
  prove.rs      joy prove (zheng artifact)

rs/             CPU backend (crate: joy-rs)
  lib.rs        pub mod formula/target/warrior
  formula.rs    bracket text <-> nox Reduction arena; subject builder
  target.rs     canonical nox TerrainConfig + versioned warrior package
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

## Implemented runtime and proof boundaries

`build` compiles `.tri` sources or projects to deterministic ProgramBundle JSON
or nox assembly. Explicit target overrides the project, then falls back to nox;
debug/release and named project profiles use one compiler path across source
build/run/prove. Publication refuses overwrite unless `--force` is explicit.
Build supports the `joy/cli/v1` result envelope. Other commands retain their
human CLI output and application exit 1 convention.

`run` accepts bundles, sources/projects and raw nox. Public inputs form the
subject list; secret call witnesses are served sequentially. `verify --claim`
without a proof is labelled re-execution, not cryptographic verification.

Proof formats and content headers:
- `zheng-nox-public-execution-v2` (`JOYEXEC2`): exact verifier-derived CCS,
  authenticated public input/output/cost, full witness disclosure and linear
  verification. No zero-knowledge or succinctness claim.
- `joy-nox-public-state-execution-v1` (`JOYST001`): actual compiled state reads,
  all namespace/key/value/root coordinates bound to verified public BBG tables.
  `--state` loads a bounded StateCertificate JSON and may pin the expected root.
- `joy-nox-ccs-triton7-zk-v3` (`JOYZK003`): Trisha's real Triton ZK checker for
  the verifier-derived Zheng relation. `--secret` or `--zk` selects this path.
  Hidden queries require all ten authenticated PUBLIC dimension tables, at
  most 2048 fields and 32768 CCS gates. Query coordinates remain private;
  this is not a hidden/private database protocol.

Execution proofs verify without re-executing nox. Unsupported dynamic
continuations/shapes and resource excess fail explicitly. See
`targets/nox/capabilities.json` and Zheng's execution specification for limits.

Legacy `prove_zheng`/`verify_zheng`/`verify_artifact` statement APIs do not prove
execution/output. CLI inspection requires `--legacy-trace-statement` and
refuses IO/state/secret constraints. Legacy recursive TensorMerkle opening
requests remain unsupported; never downgrade a rejected execution proof.

Live deployment, node/database synchronization and a private state database
remain unavailable. `cyber` is a compatible nox alias, not a live network
connection or authority to modify state.

## Execution model

- `bundle.assembly` holds the formula in bracket notation
  (`[5 [[1 3] [1 5]]]`) — exactly what `trident build --target nox` emits.
- Public words bind as `[word_last [... [word_first 0]]]`. Compiled source
  entries validate exact arity/ranges and reconstruct typed aggregate parameters
  before the ordinary body ABI; see `trident/reference/nox.md`. Native Bool input
  uses0=true/1=false. Raw nox programs retain their own subject contract.
- Secret inputs are served sequentially by `SecretProvider` to call
  patterns (tag 16).
- Legacy bundle execution uses a stack-allocated `Reduction` arena on a
  dedicated 256 MiB worker thread. Structured execution and job packing
  initialize their fixed arenas directly on the heap; explicit host limits
  select the capacity. See `specs/structured-run.md`.
- Successful pure L1 charged reductions are initial minus remaining budget;
  successful traces have that many rows. Failed traces are not a gas counter.
- `run-artifact` executes complete ART1 raw and compiler JOB1/RES1 NOXDAG01
  nouns with the bounded sequential heap evaluator and NoTrace; see
  `specs/structured-run.md` and `specs/compiler-jobs.md`.

## CLI contract

`specs/cli.md` is the CLI contract: implemented baseline plus explicitly
marked target requirements. Cross-repo jobs and acceptance are specified
in `cyber/specs/worker.md`; do not duplicate node policy in Joy.

```
joy build  <file.tri | project> [--profile NAME] [--emit bundle|nox] [-o PATH] [--force] [--format json-v1]
joy run    <bundle.json | file.tri | file.nox> [--input-values 1,2] [--secret 3] [--budget N]
joy prove  <input> [--input-values ...] [--secret ...] [--output p.zheng]
joy verify <p.zheng>                            # self-contained artifact, no re-execution
joy verify <input> --proof <p.zheng>            # same, bound to this bundle
joy verify <input> --claim <values> [--input-values ...]  # re-execution
```

`joy describe --target nox|cyber` emits a versioned JSON target package.
`targets/nox/capabilities.json` owns runtime capability declarations;
machine values come from Trident's upstream nox contract. CLI `--state`
accepts authenticated public certificates; cyber supplies no live network binding.

Build JSON-v1, describe and run-artifact have versioned machine output. Other stdout
is human-readable; stderr carries progress/diagnostics.
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
