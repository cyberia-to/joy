"""Input/preservation guards only; actual proof acceptance runs smoke-native.py on Joy."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("native_smoke", Path(__file__).with_name("smoke-native.py"))
S = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(S)


class Preservation(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.binary = self.root / "joy-fixture"
        self.binary.write_bytes(b"preflight fixture, never a proof producer")
        self.fixtures = self.root / "fixtures"
        self.fixtures.mkdir()
        for name in S.FIXTURES:
            (self.fixtures / name).write_bytes(b"{}")
        self.output = self.root / "output"

    def test_relative_binary_is_rejected_before_output(self):
        with self.assertRaisesRegex(ValueError, "absolute installed"):
            S.Smoke(Path("joy"), self.fixtures, self.output)
        self.assertFalse(self.output.exists())

    def test_existing_output_and_nested_inputs_are_preserved(self):
        self.output.mkdir()
        sentinel = self.output / "receipt.json"
        sentinel.write_bytes(b"original receipt")
        with self.assertRaisesRegex(ValueError, "fresh output"):
            S.Smoke(self.binary, self.fixtures, self.output)
        self.assertEqual(sentinel.read_bytes(), b"original receipt")
        for output in (self.fixtures / "nested", self.binary.parent / "unused" / ".." / "fixtures" / "nested"):
            with self.assertRaisesRegex(ValueError, "separate"):
                S.Smoke(self.binary, self.fixtures, output)
        self.assertFalse((self.fixtures / "nested").exists())

    def test_symlink_parent_cannot_hide_fixture_overlap(self):
        alias = self.root / "alias"
        try:
            alias.symlink_to(self.fixtures, target_is_directory=True)
        except OSError as error:
            self.skipTest(f"directory symlink unavailable: {error}")
        with self.assertRaisesRegex(ValueError, "separate"):
            S.Smoke(self.binary, self.fixtures, alias / "output")
        self.assertFalse((self.fixtures / "output").exists())

    def test_missing_or_excess_fixture_rejects_before_output(self):
        path = self.fixtures / S.FIXTURES[0]
        path.unlink()
        with self.assertRaisesRegex(ValueError, "ordinary file"):
            S.Smoke(self.binary, self.fixtures, self.output)
        path.write_bytes(b"oversize")
        with patch.object(S, "MAX_INPUT", 3):
            with self.assertRaisesRegex(ValueError, "exceeds bound"):
                S.Smoke(self.binary, self.fixtures, self.output)
        self.assertFalse(self.output.exists())

    def test_disappeared_binary_keeps_failed_receipt_and_original_reason(self):
        smoke = S.Smoke(self.binary, self.fixtures, self.output)
        smoke.report.update(status="failed", error="original execution failure")
        self.binary.unlink()
        smoke.finish()
        self.assertEqual(smoke.report["error"], "original execution failure")
        self.assertEqual(smoke.report["status"], "failed")
        self.assertIn("ordinary file", smoke.report["finalization_errors"][0])
        self.assertTrue((self.output / "receipt.json").is_file())

    def test_changed_input_cannot_leave_smoke_passed(self):
        smoke = S.Smoke(self.binary, self.fixtures, self.output)
        smoke.report["status"] = "passed"
        (self.fixtures / S.FIXTURES[0]).write_bytes(b"changed")
        smoke.finish()
        self.assertEqual(smoke.report["status"], "failed")
        self.assertIn("fixtures_end changed", smoke.report["finalization_errors"][0])

    def test_launch_failure_keeps_command_and_logs(self):
        smoke = S.Smoke(self.binary, self.fixtures, self.output)
        self.binary.unlink()
        with self.assertRaises(OSError):
            smoke.run(["prove", "missing.tri"])
        row = smoke.report["commands"][0]
        self.assertEqual(row["status"], "interrupted")
        self.assertIsNone(row["exit_code"])
        self.assertTrue((self.output / row["stdout"]).is_file())
        self.assertTrue((self.output / row["stderr"]).is_file())

    def test_ambient_compiler_overrides_are_removed_and_path_is_installed_prefix(self):
        with patch.dict(os.environ, {name: "ambient-fixture-path" for name in S.OVERRIDES}):
            smoke = S.Smoke(self.binary, self.fixtures, self.output)
        self.assertTrue(all(name not in smoke.env for name in S.OVERRIDES))
        self.assertEqual(smoke.env["PATH"], str(self.binary.parent))
        self.assertEqual(set(smoke.report["environment"]["removed_trident_overrides"]), set(S.OVERRIDES))

    def test_work_file_manifest_binds_exact_bytes_and_rejects_excess(self):
        smoke = S.Smoke(self.binary, self.fixtures, self.output)
        (smoke.work / "program.tri").write_bytes(b"original source\n")
        smoke.finish()
        self.assertEqual(smoke.report["work_files"]["program.tri"], S.identity(smoke.work / "program.tri", S.MAX_INPUT))
        for i in range(128):
            (smoke.work / str(i)).write_bytes(b"")
        smoke.finish()
        self.assertEqual(smoke.report["status"], "failed")
        self.assertIn("work file count bound", smoke.report["finalization_errors"][0])


if __name__ == "__main__":
    unittest.main()
