# CLI re-audit corrections

Baseline: Joy ea10f8ac28a6e005e9f5c81f6f85aaa0a9692fae, Trident
c15f328d8265ef128e26008f0d31076d5a0cfb45. Parent authorized both corrections.

- Batch preserves caller-selected option values by joining each valued flag and
  its value into one native OsString (`--flag=value`). Cover leading-hyphen input,
  witness/state paths, output directories and non-UTF-8 paths on Unix. This keeps
  witnesses in files and avoids shell parsing.
- Compiler-owned bounded source loading follows `../trident-wiring.md`. The
  helper owns read caps and descriptor checks, so entry points and filesystem
  imports share admission instead of adding a racy Joy preflight.
- Persist review findings under audit/soft3-reaudit/cli-review.md; keep regressions
  in cli/tests/reaudit_cli.rs and focused Trident tests. Root reviews and commits
  the two logical fixes separately. No default branch edits or dependency changes.
