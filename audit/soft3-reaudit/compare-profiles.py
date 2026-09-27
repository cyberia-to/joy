"""Compare identical native proof and compiler work across supplied Joy binaries."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
import time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--binary", action="append", required=True, help="label=path")
p.add_argument("--trident", type=Path, required=True)
p.add_argument("--output", type=Path, required=True)
a = p.parse_args()
binaries = [(label, Path(path).resolve()) for label, path in
            (v.split("=", 1) for v in a.binary)]
work = Path(tempfile.mkdtemp(prefix="joy-size-bench-"))
report = dict(work=str(work), commands=[], binaries={}, sources={}, median_ns={})
for label, binary in binaries:
    report["binaries"][label] = dict(path=str(binary), bytes=binary.stat().st_size,
        sha256=hashlib.sha256(binary.read_bytes()).hexdigest())
for repo in ["joy", "trident", "zheng", "nox", "strata", "hemera", "bbg", "lens"]:
    report["sources"][repo] = subprocess.check_output(
        ["git", "-C", str(a.trident.parent/repo), "rev-parse", "HEAD"], text=True).strip()
(work/"private.tri").write_text("program product\nfn main()->Field {let a:Field=divine()\nlet b:Field=divine()\na*b}\n")
(work/"witness.json").write_text('{"schema_version":1,"public":[],"secret":["7","13"]}')
samples = {label: {kind: [] for kind in ["prove", "verify", "compile"]} for label, _ in binaries}
hashes = set()

def run(label, binary, kind, args, measured):
    command = [str(binary), *map(str, args)]
    start = time.perf_counter_ns()
    result = subprocess.run(command, cwd=work, capture_output=True, text=True, timeout=120)
    elapsed = time.perf_counter_ns() - start
    report["commands"].append(dict(label=label, kind=kind, measured=measured,
        command=command, elapsed_ns=elapsed, exit_code=result.returncode,
        stdout=result.stdout, stderr=result.stderr))
    a.output.write_text(json.dumps(report, indent=2) + "\n")
    assert result.returncode == 0, report["commands"][-1]
    if measured:
        samples[label][kind].append(elapsed)

try:
    # One warm-up plus seven samples, reversing order to reduce ordering bias.
    # Verification consumes a proof from another build whenever available.
    for turn in range(8):
        order = binaries if turn % 2 == 0 else list(reversed(binaries))
        for label, binary in order:
            run(label, binary, "prove", ["prove", "private.tri", "--input-file", "witness.json",
                "--output", "private.zheng", "--force"], turn > 0)
        for label, binary in reversed(order):
            run(label, binary, "verify", ["verify", "private.zheng", "--claim", "91"], turn > 0)
        if turn < 4:
            for label, binary in order:
                out = work/(label + ".dag")
                run(label, binary, "compile", ["build", a.trident/"compiler/nox/main.tri",
                    "--emit", "artifact", "--artifact-profile", "compiler-job", "-o", out,
                    "--force"], turn > 0)
                hashes.add(hashlib.sha256(out.read_bytes()).hexdigest())
    assert len(hashes) == 1, "compiler artifact changed across build profiles"
    report["compiler_artifact_sha256"] = hashes.pop()
    report["samples_ns"] = samples
    report["median_ns"] = {label: {kind: int(statistics.median(values))
        for kind, values in cases.items()} for label, cases in samples.items()}
    report["status"] = "passed"
finally:
    (work/"witness.json").unlink()
    a.output.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report["median_ns"], indent=2))
