#!/usr/bin/env python3

import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]


def host_cxx():
    return (
        os.environ.get("CPPGM_HOST_CXX")
        or os.environ.get("CXX")
        or "/usr/local/opt/llvm/bin/clang++"
    )


def run(*args, **kwargs):
    kwargs.setdefault("check", True)
    kwargs.setdefault("text", True)
    return subprocess.run(args, **kwargs)


def relevant_relocation_lines(text):
    interesting_headers = (
        "Relocation information (__TEXT,__gcc_except_tab)",
        "Relocation information (__LD,__compact_unwind)",
        "Relocation information (__TEXT,__eh_frame)",
    )
    blocks = []
    current = None
    for line in text.splitlines():
        if line.startswith("Relocation information ("):
            current = [] if line.startswith(interesting_headers) else None
            if current is not None:
                blocks.append(current)
        if current is not None:
            current.append(
                re.sub(
                    r"False\s+\d+\s+\((__[^)]+)\)",
                    r"False SECTION (\1)",
                    line.rstrip(),
                )
            )
    return [line for block in sorted(blocks) for line in block]


class MachineObjectHostEhRoundtripTests(unittest.TestCase):
    def test_macho_host_eh_sections_roundtrip(self):
        if sys.platform != "darwin":
            self.skipTest("Mach-O EH roundtrip test only applies on macOS")

        env = os.environ.copy()
        env.setdefault("CPPGM_TEST_RUNNER", "0")
        run(
            "make",
            "-C",
            "dev",
            "mobjroundtrip",
            f"CXX={host_cxx()}",
            f"CPPGM_HOST_CXX={host_cxx()}",
            "CPPGM_TEST_RUNNER=0",
            cwd=REPO_ROOT,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        with tempfile.TemporaryDirectory(prefix="mobj-eh-roundtrip.") as temp_dir:
            temp = Path(temp_dir)
            src = temp / "probe.cpp"
            source_obj = temp / "probe.o"
            roundtrip_obj = temp / "probe.roundtrip.o"
            src.write_text(
                "int f(){ throw 7; }\n"
                "int g(){ try { return f(); } catch(int x) { return x; } }\n"
            )

            run(
                host_cxx(),
                "-std=gnu++11",
                "-c",
                "-o",
                str(source_obj),
                str(src),
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            run(
                str(REPO_ROOT / "dev" / "mobjroundtrip"),
                str(source_obj),
                str(roundtrip_obj),
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            source_sections = run(
                "otool",
                "-l",
                str(source_obj),
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            ).stdout
            roundtrip_sections = run(
                "otool",
                "-l",
                str(roundtrip_obj),
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            ).stdout
            for expected in ("__gcc_except_tab", "__compact_unwind", "__eh_frame"):
                self.assertIn(expected, source_sections)
                self.assertIn(expected, roundtrip_sections)

            source_relocs = run(
                "otool",
                "-rv",
                str(source_obj),
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            ).stdout
            roundtrip_relocs = run(
                "otool",
                "-rv",
                str(roundtrip_obj),
                cwd=REPO_ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            ).stdout

            self.assertEqual(
                relevant_relocation_lines(roundtrip_relocs),
                relevant_relocation_lines(source_relocs),
            )


if __name__ == "__main__":
    unittest.main()
