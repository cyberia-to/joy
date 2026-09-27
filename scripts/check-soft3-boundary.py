#!/usr/bin/env python3
"""Fail if any resolved Joy package/feature reintroduces a foreign VM backend."""
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
command = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--all-features"]
metadata = json.loads(subprocess.check_output(command, cwd=root))
for package in metadata["packages"]:
    name = package["name"]
    if name in {"trisha", "twenty-first", "bfieldcodec_derive"} or name.startswith(("trisha-", "triton-", "tasm-", "neptune")):
        raise SystemExit(f"foreign dependency in Joy: {name}")
compiler = next(p for p in metadata["packages"] if p["name"] == "trident-lang")
resolved = next(n for n in metadata["resolve"]["nodes"] if n["id"] == compiler["id"])
if "external-targets" in resolved["features"]:
    raise SystemExit("Joy enables external compiler targets/resources")
print(json.dumps({"ok": True, "packages": len(metadata["packages"]),
                  "compiler_features": resolved["features"], "command": command}))
