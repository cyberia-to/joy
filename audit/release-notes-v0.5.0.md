**Proofs bind the computation to its public result.** Joy 0.5 makes verification check the supported nox execution described by the program, inputs, selected path and output. It provides explicit public, private and authenticated-state proof modes, integrated with Trident 0.3.0 and Trisha 0.3.0.

1. **Execution becomes part of the verified statement.** The verifier derives the Zheng relation from the canonical nox program and checks its constraints, complete public input/output and selected-path cost. This closes the gap between a valid trace statement and evidence that the requested computation produced the claimed result.
2. **Public and private guarantees are explicit.** Public execution certificates disclose the complete witness and check every constraint. Private execution uses Trisha's randomized Triton 7 STARK over a checker derived from the same relation, keeping private columns out of the artifact. Neither mode can silently fall back to a legacy statement or a different checker.
3. **State reads are bound to the computation.** Each public lookup authenticates its namespace, index, returned value and complete state root. Private queries hide the selected coordinates inside the proved relation over bounded public state tables. This prevents substituting another lookup or state root while preserving a superficially valid result.
4. **Proofs enforce the source entry signature and actual control flow.** Exact input arity, aggregate layout and Bool/U32 ranges are constrained within the computation. Selected terminal branches, supported bounded loops and imported state helpers retain their return values and state requirements. Errors in inactive branches do not invalidate the branch that actually executes.
5. **Verification preserves the user's requested identity.** Source/project, target and profile stay attached to artifacts and versioned command results. Verification rejects changed programs, public inputs, outputs and state; selected-path budgets are enforced. This makes proof exchange useful across fresh processes and machines.
6. **Joy ships as part of a usable native toolchain.** macOS, Linux and Windows archives cover ARM64 and x64. Each includes Joy, Trident, the language server and Trisha, with the runtime resources needed outside a development checkout. Deterministic archives, checksums and validation receipts identify the exact files being released.

**Install:** extract the archive for your OS and CPU and add `cyber-tools/bin` to PATH. Keep all four executables together. Windows binaries statically link the MSVC runtime. A coordinated source archive is also provided; source builds require access to locked upstream Cargo dependencies.

<!-- RELEASE_DOWNLOADS -->
| Platform | ARM64 | x64 |
|---|---|---|
| macOS | [Download](https://github.com/cyberia-to/joy/releases/download/v0.5.0/cyber-tools-aarch64-apple-darwin.tar.gz) | [Download](https://github.com/cyberia-to/joy/releases/download/v0.5.0/cyber-tools-x86_64-apple-darwin.tar.gz) |
| Linux (glibc) | [Download](https://github.com/cyberia-to/joy/releases/download/v0.5.0/cyber-tools-aarch64-unknown-linux-gnu.tar.gz) | [Download](https://github.com/cyberia-to/joy/releases/download/v0.5.0/cyber-tools-x86_64-unknown-linux-gnu.tar.gz) |
| Windows | [Download](https://github.com/cyberia-to/joy/releases/download/v0.5.0/cyber-tools-aarch64-pc-windows-msvc.zip) | [Download](https://github.com/cyberia-to/joy/releases/download/v0.5.0/cyber-tools-x86_64-pc-windows-msvc.zip) |
<!-- /RELEASE_DOWNLOADS -->

**Upgrade:** update to Trident 0.3.0 / compiler API 3 and Trisha 0.3.0 together. Regenerate old private proofs for `joy-nox-ccs-triton7-zk-v3`, and regenerate BBG state certificates for state commitment v2 / opening v3. Legacy trace artifacts require explicit inspection opt-in and are labelled as not proving execution or public output.

**Scope:** public certificates are transparent and have linear verification cost. Private proofs support the documented bounded relation; private query coordinates do not make the database private. Dynamic continuations, variable branch shapes and live node/database integration remain unsupported. This is the default CPU release.

<!-- RELEASE_VALIDATION -->
**Validation:** all six native targets pass the CPU suites, all 133 execution fixtures, installed proof/certificate smoke and native process/file probes. Every producer's corpus verifies on every consumer: **36 platform pairs, 1,692 checks**, including rejected mutations. The final macOS ARM64 binaries additionally generated and verified **198 fresh baseline proofs**, covering all 43 hand-written programs; this full proof run was measured once on the dedicated 48 GiB worker. The exact released Linux ARM64 client passed admission and rejection checks against the isolated pinned Neptune node.

See [native builds](https://github.com/cyberia-to/trisha/actions/runs/35116488433), [cross-platform verification](https://github.com/cyberia-to/trisha/actions/runs/35125655070), the [validation summary](https://github.com/cyberia-to/joy/releases/download/v0.5.0/release-validation.json), [complete logs, receipts and proof corpora](https://github.com/cyberia-to/joy/releases/download/v0.5.0/release-validation.tar.gz), and [SHA-256 checksums](https://github.com/cyberia-to/joy/releases/download/v0.5.0/SHA256SUMS). These records bind validation to the released source and binary hashes.
<!-- /RELEASE_VALIDATION -->

Source commit: `d4d8e3dd587ba8d246f4905c9bb24ce3f75429a3`.
