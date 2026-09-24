"""Reproduce installed Joy compiler transport acceptance; no source compiler claim."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.joy.resolve()
    repo = Path(__file__).resolve().parents[2]
    vectors = json.loads((repo / "cli/tests/compiler_vectors.json").read_text())
    rows = []
    with tempfile.TemporaryDirectory(prefix="joy-compiler-acceptance-") as directory:
        root = Path(directory)
        for name, value in vectors["files"].items():
            (root / name).write_bytes(bytes.fromhex(value))
        output = root / "published.dag"

        def run(program, source, extra=()):
            command = [str(binary), "run-artifact", str(root / program), "--input",
                       str(root / source), "-o", str(output), "--force", *extra]
            completed = subprocess.run(command, capture_output=True, text=True, check=False)
            row = {"command": command, "exit_code": completed.returncode,
                   "stdout": completed.stdout, "stderr": completed.stderr}
            rows.append(row)
            return completed, row

        for case in vectors["cases"]:
            output.write_bytes(b"previous output")
            completed, row = run(case["program"], case["input"])
            if "error" in case:
                assert completed.returncode == 1, row
                assert not completed.stdout and case["error"] in completed.stderr, row
                assert output.read_bytes() == b"previous output", row
                row["previous_output_preserved"] = True
            else:
                assert completed.returncode == 0, row
                assert output.read_bytes() == (root / case["result"]).read_bytes(), row
                row["matches_independent_reference_bytes"] = True
                row["execution"] = json.loads(completed.stdout)["execution"]
                completed, row = run(case["program"], case["input"], ["--emit", "program"])
                if "generated" in case:
                    assert completed.returncode == 0, row
                    assert output.read_bytes() == (root / case["generated"]).read_bytes(), row
                    row["matches_independent_reference_bytes"] = True
                    row["execution"] = json.loads(completed.stdout)["execution"]
                    (root / "extracted").write_bytes(output.read_bytes())
                    completed, row = run("extracted", "zero")
                    assert completed.returncode == 0, row
                    assert output.read_bytes() == (root / "fourteen").read_bytes(), row
                    row["matches_independent_reference_bytes"] = True
                    row["execution"] = json.loads(completed.stdout)["execution"]
                else:
                    assert completed.returncode == 1 and not completed.stdout, row
                    assert "guest compilation failed" in completed.stderr, row
                    assert output.read_bytes() == (root / case["result"]).read_bytes(), row
                    row["previous_output_preserved"] = True
            assert not any(p.name.startswith(".") for p in root.iterdir())

    args.output.parent.mkdir(parents=True, exist_ok=True)
    receipt = {"schema": "joy/compiler-transport-validation/v1",
               "claim": "guest result construction and extracted-program execution; no source compilation",
               "command": ["python3", str(Path(__file__).resolve()), "--joy", str(binary), "--output", str(args.output)],
               "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
               "commands": rows}
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"commands": len(rows), "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
