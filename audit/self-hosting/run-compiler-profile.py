"""Installed CLI source -> compiler ART1 -> JOB1/RES1 -> executable ART1."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--joy", type=Path, required=True)
    p.add_argument("--fixtures", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    binary = args.joy.resolve()
    root = args.fixtures.resolve()
    rows = []

    def run(arguments, expected=None):
        command = [str(binary), *map(str, arguments)]
        r = subprocess.run(command, capture_output=True, text=True, check=False)
        row = {"command": command, "exit_code": r.returncode,
               "stdout": r.stdout, "stderr": r.stderr}
        rows.append(row)
        if expected:
            assert r.returncode == 0, row
            actual, reference = expected
            assert actual.read_bytes() == reference.read_bytes(), row
            row["matches_independent_reference_bytes"] = True
        return r, row

    seed = root / "cli-seed.dag"
    program = root / "cli-program.dag"
    result = root / "cli-result.dag"
    common = ["--emit", "artifact", "--artifact-profile", "compiler-job", "--format", "json-v1"]
    run(["build", root / "compiler.tri", *common, "-o", seed, "--force"],
        (seed, root / "compiler.dag"))
    run(["run-artifact", seed, "--input", root / "job.dag", "-o", result, "--force"],
        (result, root / "expected-result.dag"))
    run(["run-artifact", seed, "--input", root / "job.dag", "--emit", "program", "-o", program, "--force"],
        (program, root / "expected-program.dag"))
    output = root / "cli-fourteen.dag"
    completed, row = run(["run-artifact", program, "--input", root / "zero.dag", "-o", output, "--force"])
    assert completed.returncode == 0, row
    repo = Path(__file__).resolve().parents[2]
    vectors = json.loads((repo / "cli/tests/compiler_vectors.json").read_text())
    assert output.read_bytes() == bytes.fromhex(vectors["files"]["fourteen"]), row
    row["matches_independent_reference_bytes"] = True
    saved = result.read_bytes()
    completed, row = run(["run-artifact", seed, "--input", root / "zero.dag", "-o", result, "--force"])
    assert completed.returncode == 1 and not completed.stdout, row
    assert "compiler job admission" in completed.stderr, row
    assert result.read_bytes() == saved, row
    row["previous_output_preserved"] = True
    assert not any(path.name.startswith(".") for path in root.iterdir())
    receipt = {"schema": "joy/compiler-profile-validation/v1",
               "claim": "source guest constructs bound results; no source compilation by the guest",
               "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "commands": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(rows), "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
