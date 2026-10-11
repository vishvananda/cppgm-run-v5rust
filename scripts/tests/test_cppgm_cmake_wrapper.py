#!/usr/bin/env python3

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
WRAPPER = REPO_ROOT / "scripts" / "cppgm-cmake-wrapper.sh"


class CppgmCmakeWrapperTests(unittest.TestCase):
    def run_wrapper_with_fake_compiler(self, *args):
        with tempfile.TemporaryDirectory(prefix="cppgm-cmake-wrapper.") as temp_dir:
            temp = Path(temp_dir)
            argv_file = temp / "argv.txt"
            fake_compiler = temp / "fake-cppgm++"
            fake_compiler.write_text(
                "#!/usr/bin/env sh\n"
                "printf '%s\\n' \"$@\" > \"$CPPGM_WRAPPER_ARGV_FILE\"\n"
            )
            fake_compiler.chmod(0o755)

            env = os.environ.copy()
            env["CPPGM_CMAKE_COMPILER"] = str(fake_compiler)
            env["CPPGM_WRAPPER_ARGV_FILE"] = str(argv_file)

            result = subprocess.run(
                [str(WRAPPER), *args],
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            forwarded = argv_file.read_text().splitlines() if argv_file.exists() else []
            return result, forwarded

    def test_linux_no_pie_link_flag_is_ignored(self):
        result, forwarded = self.run_wrapper_with_fake_compiler(
            "-no-pie",
            "-o",
            "program",
            "input.o",
        )

        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertEqual(forwarded, ["-o", "program", "input.o"])

    def test_unknown_dash_option_still_fails(self):
        result, forwarded = self.run_wrapper_with_fake_compiler(
            "-unsupported-wrapper-probe",
            "input.o",
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported option", result.stderr)
        self.assertEqual(forwarded, [])


if __name__ == "__main__":
    unittest.main()
