"""Closed template assertions must not be reevaluated for every instance."""

import pathlib
import re
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPILER = ROOT / "dev" / "cppgm++"


@unittest.skipUnless(COMPILER.is_file(), "build dev/cppgm++ first")
class TemplateAssertionReuseTests(unittest.TestCase):
    def evaluations(self, declarations, instances):
        assertions = "\n".join(
            f'static_assert({i} + 1 == {i + 1}, "fixed");'
            for i in range(declarations)
        )
        uses = "\n".join(f"Value<{i + 1}> v{i};" for i in range(instances))
        source = (
            f"template<int N> struct Value {{ {assertions} }};\n"
            f"{uses}\nint main() {{ return 0; }}\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            source_path = pathlib.Path(directory) / "assertions.cpp"
            source_path.write_text(source)
            result = subprocess.run(
                [str(COMPILER), "--emit-lowir", "-O0", "--stats", "-o",
                 str(pathlib.Path(directory) / "out.lowir"), str(source_path)],
                capture_output=True, text=True, timeout=30, check=True,
            )
        match = re.search(r"\bconstant_evaluations=(\d+)", result.stderr)
        self.assertIsNotNone(match, result.stderr)
        return int(match.group(1))

    def test_shared_conditions_are_independent_of_instance_count(self):
        small = self.evaluations(24, 1) - self.evaluations(0, 1)
        large = self.evaluations(24, 32) - self.evaluations(0, 32)
        self.assertGreater(small, 0)
        self.assertEqual(large, small)

    def test_additional_shared_conditions_are_checked_once(self):
        single_delta = self.evaluations(32, 1) - self.evaluations(8, 1)
        repeated_delta = self.evaluations(32, 32) - self.evaluations(8, 32)
        self.assertGreater(single_delta, 0)
        self.assertEqual(repeated_delta, single_delta)


if __name__ == "__main__":
    unittest.main()
