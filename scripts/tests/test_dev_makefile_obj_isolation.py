#!/usr/bin/env python3

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
DEV_DIR = REPO_ROOT / "dev"


def host_cxx():
    return (
        os.environ.get("CPPGM_HOST_CXX")
        or os.environ.get("CXX")
        or "/usr/local/opt/llvm/bin/clang++"
    )


class DevMakefileObjIsolationTests(unittest.TestCase):
    def test_shared_obj_root_rejects_non_host_compiler(self):
        with tempfile.TemporaryDirectory(prefix="dev-makefile-fake-selfhost.") as temp_dir:
            fake_cxx = Path(temp_dir) / "fake-selfhost-cxx"
            fake_cxx.write_text("#!/bin/sh\nexit 0\n")
            fake_cxx.chmod(0o755)

            result = subprocess.run(
                [
                    "make",
                    "-C",
                    str(DEV_DIR),
                    "-n",
                    "cppgm++",
                    f"CXX={fake_cxx}",
                    f"CPPGM_HOST_CXX={host_cxx()}",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("use OBJ=<isolated-root>", result.stderr)

    def test_isolated_obj_root_allows_non_host_compiler(self):
        with tempfile.TemporaryDirectory(prefix="dev-makefile-isolated-obj.") as temp_dir:
            fake_cxx = Path(temp_dir) / "fake-selfhost-cxx"
            fake_cxx.write_text("#!/bin/sh\nexit 0\n")
            fake_cxx.chmod(0o755)
            isolated_obj = Path(temp_dir) / "obj-selfhost"

            result = subprocess.run(
                [
                    "make",
                    "-C",
                    str(DEV_DIR),
                    "-n",
                    "cppgm++",
                    f"CXX={fake_cxx}",
                    f"CPPGM_HOST_CXX={host_cxx()}",
                    f"OBJ={isolated_obj}",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            self.assertEqual(result.returncode, 0, msg=result.stderr)


if __name__ == "__main__":
    unittest.main()
