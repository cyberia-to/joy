# Explicit compiler arena with reproducible dependencies

Combined Joy source `1639a27a46d6bb8f51e25375ff38a36193b89a3e` consumes the accepted deterministic codec overlay.
The [receipt](explicit-compiler-arena-combined-validation.json) pins every sibling,
all seven owner gates, installed binaries and rerun CLI commands. Source limits
and all arena behavior match the [original delivery](explicit-compiler-arena.md).

1165 Trident, 123 Joy and 380 Trisha tests pass with zero warnings; four existing
Trisha tests remain ignored. All 133 benchmark rows and 43 manual baselines match.

The unchanged 3004-byte compiler fixture again compiles under the explicit
3145728-node / 100000000-reduction / 300000ms host policy: 1629101 allocated nodes,
96742035 reductions and 1400 peak frames. Its complete generated ART1 and all
execution metadata/costs, excluding elapsed time, match the original receipt.
The 18-command behavioral run checks four fresh source jobs and two rejections;
the 8-command boundary run retains old-ceiling rejection, 1629102-node exact
success and 1629101-node failure, preserving complete output files.

The codec prerequisite contains its separate fresh-build reproduction. This
root records committed cached installation and fresh execution of every
arena-specific corpus. Full compiler closure, C2/C3, six-platform acceptance
and native proof gates remain open.
