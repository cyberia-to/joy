# Portable installed native proof smoke

All four existing bounded proof routes passed on one installed Joy on macOS ARM:
49 commands, comprising 21 successes and 28 expected rejections. The independent
outputs were 24 (public arithmetic), 91 (private arithmetic), and 82 for both
public and private state reads. Each proof was checked in fresh verifier
processes, both self-contained and bound to its source. This is an installed
binary regression; dynamic compiler proofs and SH7/SH8 remain separate.

The Python standard-library runner accepts an absolute installed binary path,
three public certificate fixtures and a fresh output directory:

```sh
cargo run --release --locked --offline -p joy-rs --example release_state_fixtures -- /absolute/fresh-fixtures
python3 -B scripts/smoke-native.py --joy /absolute/install/bin/joy --fixtures /absolute/fresh-fixtures --output /absolute/fresh-smoke
```

Use `joy.exe` on Windows. Python 3.10 or later is required. The fixture example
uses the selected BBG build. The smoke itself runs only the supplied Joy, performs
no build, and invokes no Trisha or platform-specific profiler. It removes the
four Trident library/target override variables and restricts child `PATH` to the
installed binary's directory; the receipt records that environment selection.
Other inherited OS environment variables remain available.

The routes require `JOYEXEC2`, `JOYST001` and `JOYZH001` headers. They check
independent claims, changed public input, changed authenticated state, semantic
source changes, appended proof bytes, fresh private randomness and verifier
witness refusal. Zero, one and excess secrets, incomplete private state and
overwrite refusal preserve an existing output. Explicit replacement is then
proved and verified. Retired private headers and foreign targets are refused.
Each command has a 120-second timeout. File-size and retained-log bounds are
checked after commands; they are not active subprocess disk quotas.

The measurement used an isolated nine-repository family: Trident `57491633`,
Joy base `ec83bd8d` plus these three added source files, Nox `172811b7`, Zheng
`633e5ba9`, and the five library revisions recorded in [receipt.json](receipt.json).
Those nine repositories and the added source hashes were identical before and
after the final run. The Joy worktree additions were uncommitted at measurement;
the other eight inputs were clean. This family includes later Nox observer and
Zheng component integration, so it is distinct from the frozen SH6 family.
The Joy base comes from PR22. No six-platform, SH6 or release verdict follows
from this local result.

Installed Joy SHA-256 was
`d806e3018ac2b2f7dfbee52de505da8d467bc7cac3c4c65fa5dcf9ecd3a9962b`.
The exact source-pinned final driver, commands, Rust version, logs and start/end
identities are retained in `final/validation.json` inside the archive. They cover
the soft3 dependency boundary, release all-target check, release workspace tests
(172 passed, zero failed or ignored; 27 summaries), formatting of the new Rust
example, and nine Python preservation guards under normal and optimized Python.
All final commands exited zero with zero warnings. The Python guards do not
simulate successful proof acceptance; the separate installed smoke does that.

The initial setup attempt is retained under `initial/`. Cargo install exited
zero but printed its missing-PATH advisory, so the driver stopped on its warning
rule before running a smoke or Rust test. The final build driver prepended the
installed directory to its own `PATH`; it did not suppress diagnostics. The
initial source hash map and logs are retained, but that initial Python source
revision was not captured in full. It supplies no proof-acceptance evidence.

[raw-evidence.tar.gz](raw-evidence.tar.gz) retains 166 ordinary files, 5,556,867
uncompressed bytes, with exact per-file lengths and hashes in
[files.json](files.json). This includes all 49 command logs, 22 work files
(sources, inputs and proofs), fixture inputs, the final measured source copies,
both drivers and the collector. Archive SHA-256 is
`d10947aff332a1c29120cff825bd17cceb9b1e980a7d26c2d37358d3b4869175`.
The collector reopened the archive and compared every member byte with its
original. To inspect it, extract into a fresh directory with
`python3 -m tarfile -e raw-evidence.tar.gz /absolute/fresh-evidence` and check its
members against the index. Recorded absolute paths describe the measured local
invocations; the portable smoke accepts new explicit paths on another host.
