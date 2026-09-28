"""Exercise an installed Joy's four bounded proof routes; no build or runtime fallback.

Public state fixtures are generated beforehand by release_state_fixtures.
This smoke does not prove dynamic compilation or establish SH7/SH8 acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

SCHEMA = "joy/installed-native-smoke/v1"
FIXTURES = ("state.json", "state-all.json", "other-state.json")
FORMATS = ("zheng-nox-public-execution-v2", "joy-nox-public-state-execution-v1",
           "joy-nox-zheng-private-execution-v1")
TIMEOUT = 120
MAX_INPUT, MAX_LOG, MAX_PROOF = 16 << 20, 4 << 20, 256 << 20
OVERRIDES = ("TRIDENT_STDLIB", "TRIDENT_OSLIB", "TRIDENT_EXTLIB", "TRIDENT_TARGET_PACKAGES")
SOURCES = {
    "public.tri": "program arithmetic\nfn main(x:Field,y:Field)->Field {(x+y)*x}\n",
    "private.tri": "program product\nfn main()->Field {let a:Field=divine()\n let b:Field=divine()\n a*b}\n",
    "state.tri": "program state_test\nfn helper(k:Field)->Field {os.state.read(k)}\nfn main(k:Field)->Field {helper(k)+5}\n",
    "private-state.tri": "program hidden_lookup\nfn helper(k:Field)->Field {os.state.read(k)}\nfn main()->Field {let k:Field=divine()\n helper(k)+5}\n",
}
ALTERATIONS = {"public.tri": ("(x+y)*x", "(x+y)*x+1"),
               "private.tri": ("a*b}", "a*b+1}"),
               "state.tri": ("helper(k)+5", "helper(k)+6"),
               "private-state.tri": ("helper(k)+5", "helper(k)+6")}


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def identity(path, limit):
    require(path.is_file() and not path.is_symlink(), f"ordinary file required: {path}")
    size = path.stat().st_size
    require(size <= limit, f"file exceeds bound: {path}")
    return dict(bytes=size, sha256=sha(path))


def separate(output, inputs):
    for path in inputs:
        path = path.resolve()
        require(not output.is_relative_to(path) and not path.is_relative_to(output),
                "output must be separate from installed binary and fixtures")


class Smoke:
    def __init__(self, binary, fixtures, output):
        require(binary.is_absolute(), "--joy must be an absolute installed binary path")
        self.binary, self.fixtures, self.output = binary, fixtures.resolve(), output.resolve()
        require(not output.is_symlink() and not self.output.exists(), "fresh output required")
        separate(self.output, (binary, fixtures))
        binary_identity = identity(binary, MAX_PROOF)
        fixture_identities = {name: identity(self.fixtures / name, MAX_INPUT) for name in FIXTURES}
        self.output.mkdir(parents=True)
        self.work = self.output / "work"
        self.work.mkdir()
        (self.output / "commands").mkdir()
        self.env = dict(os.environ)
        removed = {name: self.env.pop(name) for name in OVERRIDES if name in self.env}
        self.env["PATH"] = str(binary.parent)
        self.report = dict(schema=SCHEMA, status="running", started_ns=time.time_ns(),
                           scope="Four existing bounded execution-proof routes on supplied Joy; dynamic compiler proofs separate",
                           binary=str(binary), binary_start=binary_identity, fixtures=str(self.fixtures),
                           fixtures_start=fixture_identities, script_start=identity(Path(__file__).resolve(), MAX_INPUT),
                           host=dict(system=platform.system(), machine=platform.machine(),
                                     platform=platform.platform(), libc=platform.libc_ver(), python=sys.version),
                           environment=dict(path=self.env["PATH"], removed_trident_overrides=removed),
                           commands=[], routes={}, artifacts={})
        self.flush()

    def flush(self):
        temporary = self.output / "receipt.next.json"
        temporary.write_text(json.dumps(self.report, indent=2) + "\n", encoding="utf-8", newline="\n")
        temporary.replace(self.output / "receipt.json")

    def run(self, arguments, expected=0, diagnostic=None):
        command = [str(self.binary), *map(str, arguments)]
        number = len(self.report["commands"])
        row = dict(argv=command, cwd=str(self.work), expected_exit=expected, exit_code=None,
                   status="running", stdout=f"commands/{number}.stdout", stderr=f"commands/{number}.stderr")
        self.report["commands"].append(row)
        self.flush()
        started = time.monotonic_ns()
        with (self.output / row["stdout"]).open("xb") as out, (self.output / row["stderr"]).open("xb") as err:
            try:
                result = subprocess.run(command, cwd=self.work, env=self.env, stdout=out, stderr=err, timeout=TIMEOUT)
                row.update(status="completed", exit_code=result.returncode)
            except BaseException:
                row["status"] = "interrupted"
                raise
            finally:
                row["elapsed_ns"] = time.monotonic_ns() - started
                self.flush()
        for stream in ("stdout", "stderr"):
            row[stream + "_identity"] = identity(self.output / row[stream], MAX_LOG)
        self.flush()
        stdout = (self.output / row["stdout"]).read_bytes()
        stderr = (self.output / row["stderr"]).read_bytes()
        require(result.returncode == expected, f"command {number}: expected exit {expected}, got {result.returncode}")
        if expected:
            require(stderr, f"command {number}: rejection diagnostic required")
        if diagnostic:
            require(diagnostic in stderr, f"command {number}: intended rejection diagnostic missing")
        return stdout

    def proof(self, source, name, inputs, magic):
        self.run(["prove", source, "--profile", "release", *inputs, "--output", name])
        path = self.work / name
        info = identity(path, MAX_PROOF)
        with path.open("rb") as stream:
            require(stream.read(8) == magic, f"wrong proof profile: {name}")
        self.report["artifacts"][name] = info
        self.flush()

    def route(self, name, source, inputs, public, claim, magic, state=None):
        require(self.run(["run", source, "--profile", "release", *inputs]) == f"{claim}\n".encode(),
                f"independent output differs: {name}")
        proof = name + ".zheng"
        self.proof(source, proof, inputs, magic)
        expected = ["--claim", str(claim)]
        if public:
            expected += ["--input-values", public]
        if state:
            expected += ["--state", state]
        self.run(["verify", proof, *expected])
        self.run(["verify", source, "--profile", "release", "--proof", proof, *expected])
        self.run(["verify", proof, "--claim", str(claim + 1)], 1, b"Verification: FAIL")
        if public:
            values = public.split(",")
            values[0] = str(int(values[0]) + 1)
            self.run(["verify", proof, "--input-values", ",".join(values)], 1, b"Verification: FAIL")
        if state:
            self.run(["verify", proof, "--state", "other-state.json"], 1, b"Verification: FAIL")
        changed = "changed-" + source
        before, after = ALTERATIONS[source]
        require(SOURCES[source].count(before) == 1, "one semantic source alteration required")
        (self.work / changed).write_bytes(SOURCES[source].replace(before, after).encode("utf-8"))
        self.run(["verify", changed, "--profile", "release", "--proof", proof, *expected], 1, b"Verification: FAIL")
        corrupted = "corrupt-" + proof
        with (self.work / proof).open("rb") as original, (self.work / corrupted).open("xb") as output:
            shutil.copyfileobj(original, output, 1 << 20)
            output.write(b"\0")
        self.run(["verify", corrupted], 1, b"Verification: FAIL")
        self.report["routes"][name] = dict(source=source, proof=proof, expected_output=claim,
                                            public_inputs=public, state=state, header=magic.decode())
        self.flush()

    def execute(self):
        for name in FIXTURES:
            shutil.copyfile(self.fixtures / name, self.work / name)
            require(identity(self.work / name, MAX_INPUT) == self.report["fixtures_start"][name], "copied fixture identity")
        for name, source in SOURCES.items():
            (self.work / name).write_bytes(source.encode("utf-8"))
        package = json.loads(self.run(["describe", "--target", "nox"]))
        require(package["runtime"]["proof_formats"] == list(FORMATS), "unexpected proof capability profile")
        require(package["runtime"]["deploy"] is False, "unexpected live deployment capability")
        self.route("public", "public.tri", ["--input-values", "3,5"], "3,5", 24, b"JOYEXEC2")
        self.route("private", "private.tri", ["--secret", "7,13"], None, 91, b"JOYZH001")
        self.route("state", "state.tri", ["--state", "state-all.json", "--input-values", "11"],
                   "11", 82, b"JOYST001", "state-all.json")
        self.route("private-state", "private-state.tri", ["--state", "state-all.json", "--secret", "11"],
                   None, 82, b"JOYZH001", "state-all.json")
        self.proof("private.tri", "private-fresh.zheng", ["--secret", "7,13"], b"JOYZH001")
        require(sha(self.work / "private.zheng") != sha(self.work / "private-fresh.zheng"), "fresh private randomness required")
        self.run(["verify", "private-fresh.zheng", "--claim", "91"])
        self.run(["verify", "private.zheng", "--secret", "7,13"], 1, b"Verification: FAIL")
        self.run(["verify", "public.zheng", "--budget", "1"], 1, b"Verification: FAIL")
        preserved = self.work / "preserved.zheng"
        preserved.write_bytes(b"prior publication\n")
        for arguments in ([], ["--secret", "7"], ["--secret", "7,13,42"]):
            self.run(["prove", "private.tri", "--profile", "release", *arguments,
                      "--output", preserved.name, "--force"], 1)
            require(preserved.read_bytes() == b"prior publication\n", "failed proof replaced existing output")
        self.run(["prove", "state.tri", "--state", "state.json", "--input-values", "11", "--zk",
                  "--output", preserved.name, "--force"], 1)
        require(preserved.read_bytes() == b"prior publication\n", "partial private state replaced output")
        self.run(["prove", "private.tri", "--profile", "release", "--secret", "7,13",
                  "--output", preserved.name], 1)
        require(preserved.read_bytes() == b"prior publication\n", "overwrite refusal changed output")
        self.run(["prove", "private.tri", "--profile", "release", "--secret", "7,13",
                  "--output", preserved.name, "--force"])
        self.run(["verify", preserved.name, "--claim", "91"])
        for header in (b"JOYZK001", b"JOYZK002", b"JOYZK003"):
            (self.work / "retired.zheng").write_bytes(header)
            self.run(["verify", "retired.zheng"], 1)
        for target in ("triton", "neptune"):
            self.run(["describe", "--target", target], 1)
        require(not any(path.name.startswith(".") for path in self.work.iterdir()), "publication staging file remains")
        self.report["status"] = "passed"

    def finish(self):
        try:
            files, total = {}, 0
            for path in sorted(self.work.iterdir()):
                require(len(files) < 128, "work file count bound")
                info = identity(path, MAX_PROOF)
                total += info["bytes"]
                require(total <= 1 << 30, "retained work byte bound")
                files[path.name] = info
            self.report["work_files"] = files
        except BaseException as error:
            self.report.setdefault("finalization_errors", []).append(f"{type(error).__name__}: {error}")
            self.report["status"] = "failed"
        for key, callback in (
            ("binary_end", lambda: identity(self.binary, MAX_PROOF)),
            ("fixtures_end", lambda: {n: identity(self.fixtures / n, MAX_INPUT) for n in FIXTURES}),
            ("script_end", lambda: identity(Path(__file__).resolve(), MAX_INPUT)),
        ):
            try:
                self.report[key] = callback()
                require(self.report[key] == self.report[key.replace("_end", "_start")], f"{key} changed")
            except BaseException as error:
                self.report.setdefault("finalization_errors", []).append(f"{type(error).__name__}: {error}")
                self.report["status"] = "failed"
        self.report["ended_ns"] = time.time_ns()
        self.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--joy", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    smoke = Smoke(args.joy, args.fixtures, args.output)
    try:
        smoke.execute()
    except BaseException as error:
        smoke.report.update(status="failed", error=f"{type(error).__name__}: {error}")
    finally:
        smoke.finish()
    print(json.dumps(dict(status=smoke.report["status"], receipt=str(smoke.output / "receipt.json"))))
    return 0 if smoke.report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
