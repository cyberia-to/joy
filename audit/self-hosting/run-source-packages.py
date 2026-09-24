"""Installed exact-file packaging and guest transport acceptance."""
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
    with tempfile.TemporaryDirectory(prefix="joy-pack-acceptance-") as directory:
        root = Path(directory)
        for name in ["compiler", "job", "generated", "zero", "fourteen"]:
            (root / name).write_bytes(bytes.fromhex(vectors["files"][name]))
        (root / "source.bytes").write_bytes(b"raw\0\r\n\xff")
        manifest = {"version": 1, "entry_module": "demo", "entry_function": "main",
                    "modules": [{"logical_path": "demo", "file": "source.bytes", "origin_name": "fixture", "origin_version": "1"}],
                    "options": {"target": 0, "input_profile": 0, "output_profile": 0, "optimization": 0, "cfg_flags": ["release"]},
                    "limits": {"source_bytes": 4096, "modules": 32, "diagnostics": 16, "sequence_length": 4096,
                               "validation_visits": 100000, "artifact_bytes": 1048576, "artifact_nodes": 3000,
                               "artifact_depth": 128, "reductions": 1000000, "arena_nodes": 3000, "evaluator_frames": 16384}}
        (root / "package.json").write_text(json.dumps(manifest))

        def run(arguments, expected=None):
            command = [str(binary), *map(str, arguments)]
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            row = {"command": command, "exit_code": result.returncode,
                   "stdout": result.stdout, "stderr": result.stderr}
            rows.append(row)
            if expected:
                assert result.returncode == 0, row
                actual, reference = expected
                assert actual.read_bytes() == reference.read_bytes(), row
                row["matches_independent_reference_bytes"] = True
            return result, row

        packed = root / "packed.dag"
        pack = ["pack-job", "--compiler", root / "compiler", "--manifest", root / "package.json", "-o", packed]
        run(pack, (packed, root / "job"))
        generated = root / "program.dag"
        run(["run-artifact", root / "compiler", "--input", packed, "--emit", "program", "-o", generated],
            (generated, root / "generated"))
        output = root / "output.dag"
        run(["run-artifact", generated, "--input", root / "zero", "-o", output], (output, root / "fourteen"))
        saved = packed.read_bytes()
        manifest["limits"]["source_bytes"] = 6
        (root / "package.json").write_text(json.dumps(manifest))
        result, row = run([*pack, "--force"])
        assert result.returncode == 1 and not result.stdout, row
        assert packed.read_bytes() == saved, row
        row["previous_output_preserved"] = True
        assert not any(path.name.startswith(".") for path in root.iterdir())

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"schema": "joy/source-packages-validation/v1",
        "claim": "exact source packaging and guest transport; no guest source compilation",
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "commands": rows}, indent=2) + "\n")
    print(json.dumps({"commands": len(rows), "receipt": str(args.output)}))


if __name__ == "__main__":
    main()
