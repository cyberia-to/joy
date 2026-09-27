# Joy soft3 correctness and binary size re-audit

User request: independently audit native Joy after removal of the Neptune stack,
and explain the installed binary size with measured contributions.

Starting origin release/0.4: Joy ea10f8a, Zheng 2a9e9cc, Trident c15f328.
Use isolated sibling worktrees; no Trisha checkout. Defaults stay unchanged.

1. Revalidate resolved all-feature dependencies, embedded resources and installed
   commands, including private/state proofs and the paired Trident CLI.
2. Review Zheng soundness/privacy, Joy runtime/artifact/worker boundaries and CLI
   admission/publication independently; record reproducible findings in audit/.
   Separate known coverage limits from correctness defects. Prepare concrete fix
   scope before any implementation changes under the existing owner authorization.
3. Measure Mach-O sections, live linked code by crate/function, embedded source
   bytes and metadata. Compare controlled release-profile builds and a minimal
   Rust executable using identical toolchains. Do not equate dependency inventory
   or unstripped symbol estimates with binary-byte ownership.
4. Record exact revisions, commands, hashes and validation for each result. Test
   any proposed shipping size change against the same proof/CLI behavior. Keep
   experiments and local development evidence distinct from release promotion.

Confirmed fix scope under the existing instruction to complete native Joy:

- Joy: explicit public-proof budget; iterative bounded raw parser and DAG output
  preflight; lossless batch flag forwarding for option-looking native paths.
- Trident: compiler-owned regular-file, byte-capped source/project reads, with
  descriptor admission on supported Unix platforms. Preserve source symlinks.
- Zheng: remove measured zero-inverse timing shortcut; document remaining local
  side-channel and memory-erasure scope. Keep protocol and proof format unchanged.

Build-profile experiments remain measurements of the pinned baseline. They do
not change the shipping profile or pretend to certify all platform performance.
Each implementation fix has enabled regression tests and an atomic commit.
