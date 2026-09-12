# trident-side wiring for the joy warrior (M3)

## Foundation alignment request

`soft3/specs/execution-model.md` now owns the common architecture. Align
`trident/reference/warrior-api.md` and target discovery to it: a warrior
supports a VM/OS family across an open-ended set of compatible network
instances, and workers instantiate its capabilities using selected backends.
An external binary is one interface, not the warrior's required process shape.
Bundled network presets must remain optional; a new compatible network must
be configurable without new source enums or a binary rebuild. Keep VM/OS,
proof profile, network/genesis and executor identities distinct.

## Current request: build and deploy contract

The target command contract is now in `joy/specs/cli.md` under
"target build contract" and "target deploy contract". Both commands remain
planned. Trident-side integration should:

- Keep the reference nox lowering shared through the compiler library.
  Joy build must never shell back into a delegating `trident build`.
- Check artifact equivalence for identical compiler options, source inputs
  and target packages. Joy defaults to ProgramBundle JSON, while today's
  Trident nox build emits assembly; compare the corresponding representations.
- Distinguish registry publication in `trident/src/cli/deploy.rs` from
  warrior network deployment. Neither route may silently substitute for
  the other.
- Add an explicit deployment plan, signer and receipt API before delegating
  network deployment. The current Deployer trait lacks this context.
- Preserve `deploy=false` until actual authorized submission is supported;
  offline preparation alone must not advertise network deployment.

This is a wiring request, not a change to Trident. The M3 notes below are
historical and describe the 2026-09-07 implementation, including old paths
and capability declarations.

joy does not touch ~/cyber/trident. This file is the exact patch the
trident owner applies so `trident run/prove/verify --target nox`
delegates to joy. Verified against trident master as of 2026-09-07.

## 1. Register joy in the nox terrain (required)

File: `trident/vm/nox/target.toml` — add this section (mirror of
triton's `[warrior]` block; place it between `[cost]` and `[status]`):

```toml
[warrior]
name = "joy"
crate = "joy"
runner = true
prover = false   # flip to true in M4 when `joy prove` emits zheng proofs
```

Why this is sufficient: `trident/src/cli/mod.rs::find_warrior()` already
resolves in this order: `trident-nox` on PATH → `[warrior]` config →
`trident-joy` → bare `joy` on PATH. With the section above and
`cargo install --path cli --force` run in ~/cyber/joy, the bare `joy`
binary is found. No trident code changes are needed for delegation —
joy accepts exactly what `delegate_to_warrior` sends:

- run:   `joy run <input.tri> --target nox --profile <p> [--input-values ..] [--secret ..] [--state ..]`
- prove: `joy prove <input.tri> --target nox --profile <p> [--output ..]` → exits 1 with the M4 dash
- verify:`joy verify <proof-path> --target nox` → a `.toml`/`.proof` path gets the honest
  M4 dash; bundle/tri/nox paths verify by re-execution with `--claim`

## 2. Known limitation, pre-existing (no action for M3)

`TerrainConfig::resolve("nox")` searches `vm/nox/target.toml` relative
to the trident binary or the cwd. An installed `~/.cargo/bin/trident`
finds it only when invoked from the trident repo. joy is immune (it
carries a fallback mirror in `joy/rs/target.rs`), but
`trident run --target nox` from an arbitrary cwd will not resolve the
target and hence not find the warrior config. Fixing that (embedded
nox config, like the `triton()` builtin) is trident-owner territory.

## 3. Bug found while wiring joy (trident-owner fix, M1 scope)

`trident/src/ir/tree/lower/nox.rs:446` lowers `divine()` to
`[16 [0 [0 1]]]`. nox's call pattern (`nox/rs/patterns/call.rs`)
requires the body to be `[tag_formula check_formula]` where both are
formulas: the bare atom `0` as tag_formula is Malformed at runtime,
and `[0 1]` as check returns a pair, which nox treats as rejection.
Phase-1-correct emission:

```rust
// [16 [[1 0] [1 0]]] — tag = quote 0, check = quote 0 (always accept)
Ok(Noun::cell(
    Noun::atom(16),
    Noun::cell(
        Noun::cell(Noun::atom(1), Noun::atom(0)),
        Noun::cell(Noun::atom(1), Noun::atom(0)),
    ),
))
```

joy's SecretProvider already serves witnesses to correctly-shaped call
patterns (verified in `joy/rs/tests/integration.rs::secret_input_served_by_call_pattern`).

## 4. Sync duty

`joy/rs/target.rs` mirrors `trident/vm/nox/target.toml` (used only when
resolve fails). If the nox target.toml changes (hash, degree, cost
tables), update the mirror in the same breath.
