#!/usr/bin/env python3

import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "dump_host_eh_object_facts.pl"


def write_executable(path: Path, contents: str) -> None:
    path.write_text(contents)
    path.chmod(0o755)


class DumpHostEhObjectFactsPlTests(unittest.TestCase):
    def run_helper(self, *, magic: bytes, nm_output: str,
                   readelf_script: str = "", otool_script: str = "") -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="cppgm-host-eh-facts.") as temp_dir:
            temp = Path(temp_dir)
            obj = temp / "probe.o"
            obj.write_bytes(magic + b"\0" * 64)

            write_executable(
                temp / "nm",
                textwrap.dedent(
                    f"""\
                    #!/usr/bin/env python3
                    import sys
                    sys.stdout.write({nm_output!r})
                    """
                ),
            )
            if readelf_script:
                write_executable(temp / "readelf", readelf_script)
            if otool_script:
                write_executable(temp / "otool", otool_script)

            env = os.environ.copy()
            env["PATH"] = f"{temp}:{env.get('PATH', '')}"
            return subprocess.run(
                ["perl", str(SCRIPT), str(obj)],
                cwd=REPO_ROOT,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

    def test_macho_contract_facts_and_lsda_decode(self) -> None:
        otool_script = textwrap.dedent(
            """\
            #!/usr/bin/env python3
            import sys
            args = sys.argv[1:]
            if args[:1] == ["-l"]:
                sys.stdout.write('''
                Section
                  sectname __text
                   segname __TEXT
                      addr 0x0
                Section
                  sectname __gcc_except_tab
                   segname __TEXT
                      addr 0x40
                Section
                  sectname __compact_unwind
                   segname __LD
                      addr 0x80
                ''')
            elif args[:1] == ["-rv"]:
                sys.stdout.write('''
                Relocation information (__TEXT,__text) 2 entries
                address  pcrel length extern type    scattered symbolnum/value
                00000018 True  long   True   BRANCH  False     ___cxa_throw
                00000030 True  long   True   GOT_LD  False     __ZTI4Mark
                Relocation information (__TEXT,__gcc_except_tab) 1 entries
                address  pcrel length extern type    scattered symbolnum/value
                00000010 True  long   True   GOT     False     __ZTI4Mark
                Relocation information (__LD,__compact_unwind) 2 entries
                address  pcrel length extern type    scattered symbolnum/value
                00000030 False ?( 3)  True   UNSIGND False     ___gxx_personality_v0
                00000038 False ?( 3)  False  UNSIGND False     2 (__TEXT,__gcc_except_tab)
                ''')
            elif args[:1] == ["-s"]:
                sys.stdout.write('''
                Contents of (__TEXT,__gcc_except_tab) section
                0000000000000000 ff9b0d01 0400050a 01010000 04000000
                ''')
            """
        )
        result = self.run_helper(
            magic=b"\xcf\xfa\xed\xfe",
            nm_output=textwrap.dedent(
                """\
                0000000000000000 T _main
                                 U ___cxa_throw
                                 U ___gxx_personality_v0
                0000000000000000 S __ZTI4Mark
                """
            ),
            otool_script=otool_script,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = set(result.stdout.splitlines())
        self.assertIn("object 1 define _ZTI4Mark", lines)
        self.assertIn("object 1 undef __cxa_throw", lines)
        self.assertIn("object 1 section host_lsda", lines)
        self.assertIn("object 1 section host_unwind", lines)
        self.assertIn("object 1 reloc_call __cxa_throw", lines)
        self.assertIn("object 1 reloc_ref __gxx_personality_v0", lines)
        self.assertIn("object 1 reloc_section_ref host_lsda", lines)
        self.assertNotIn("object 1 lsda call_site_starts_at_zero", lines)
        self.assertIn("object 1 lsda action_has_catch", lines)
        self.assertIn("object 1 lsda type_table", lines)

    def test_elf_contract_facts_and_cleanup_lsda_decode(self) -> None:
        readelf_script = textwrap.dedent(
            """\
            #!/usr/bin/env python3
            import sys
            args = sys.argv[1:]
            if args[:1] == ["-SW"]:
                sys.stdout.write('''
                  [ 1] .text             PROGBITS         0000000000000000  00000040
                  [ 2] .gcc_except_table PROGBITS         0000000000000000  00000080
                  [ 3] .eh_frame         X86_64_UNWIND    0000000000000000  000000b0
                ''')
            elif args[:1] == ["-rW"]:
                sys.stdout.write('''
                Relocation section '.rela.text' at offset 0x1f0 contains 1 entries:
                    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend
                0000000000000018  0000000400000004 R_X86_64_PLT32         0000000000000000 _Unwind_Resume - 4
                Relocation section '.rela.gcc_except_table' at offset 0x220 contains 1 entry:
                    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend
                0000000000000010  0000000500000002 R_X86_64_PC32          0000000000000000 .text + 0
                Relocation section '.rela.eh_frame' at offset 0x250 contains 2 entries:
                    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend
                0000000000000020  0000000200000002 R_X86_64_PC32          0000000000000000 .text + 0
                0000000000000053  0000000600000009 R_X86_64_GOTPCREL      0000000000000000 __gxx_personality_v0 + 0
                ''')
            elif args[:1] == ["-x"]:
                sys.stdout.write('''
                Hex dump of section '.gcc_except_table':
                  0x00000000 ffff0104 00050a00 0000
                ''')
            """
        )
        result = self.run_helper(
            magic=b"\x7fELF",
            nm_output=textwrap.dedent(
                """\
                0000000000000000 T main
                                 U _Unwind_Resume
                                 U __gxx_personality_v0
                """
            ),
            readelf_script=readelf_script,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = set(result.stdout.splitlines())
        self.assertIn("object 1 undef _Unwind_Resume", lines)
        self.assertIn("object 1 section host_lsda", lines)
        self.assertIn("object 1 section host_unwind", lines)
        self.assertIn("object 1 reloc_call _Unwind_Resume", lines)
        self.assertIn("object 1 reloc_ref __gxx_personality_v0", lines)
        self.assertIn("object 1 reloc_section_ref text", lines)
        self.assertIn("object 1 lsda call_site_has_cleanup", lines)
        self.assertNotIn("object 1 lsda type_table", lines)

    def test_elf_lsda_with_fixed_width_call_site_entries_decodes(self) -> None:
        # udata4 entries (encoding 0x03) after a uleb128 table length.
        readelf_script = textwrap.dedent(
            """\
            #!/usr/bin/env python3
            import sys
            args = sys.argv[1:]
            if args[:1] == ["-SW"]:
                sys.stdout.write('''
                  [ 1] .text             PROGBITS         0000000000000000  00000040
                  [ 2] .gcc_except_table PROGBITS         0000000000000000  00000080
                  [ 3] .eh_frame         X86_64_UNWIND    0000000000000000  000000b0
                ''')
            elif args[:1] == ["-rW"]:
                sys.stdout.write('''
                Relocation section '.rela.text' at offset 0x1f0 contains 1 entries:
                    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend
                0000000000000018  0000000400000004 R_X86_64_PLT32         0000000000000000 _Unwind_Resume - 4
                Relocation section '.rela.gcc_except_table' at offset 0x220 contains 1 entry:
                    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend
                0000000000000010  0000000500000002 R_X86_64_PC32          0000000000000000 .text + 0
                Relocation section '.rela.eh_frame' at offset 0x250 contains 2 entries:
                    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend
                0000000000000020  0000000200000002 R_X86_64_PC32          0000000000000000 .text + 0
                0000000000000053  0000000600000009 R_X86_64_GOTPCREL      0000000000000000 __gxx_personality_v0 + 0
                ''')
            elif args[:1] == ["-x"]:
                sys.stdout.write('''
                Hex dump of section '.gcc_except_table':
                  0x00000000 ffff030d 00000000 05000000 0a000000
                  0x00000010 00
                ''')
            """
        )
        result = self.run_helper(
            magic=b"\x7fELF",
            nm_output=textwrap.dedent(
                """\
                0000000000000000 T main
                                 U _Unwind_Resume
                                 U __gxx_personality_v0
                """
            ),
            readelf_script=readelf_script,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = set(result.stdout.splitlines())
        self.assertIn("object 1 undef _Unwind_Resume", lines)
        self.assertIn("object 1 section host_lsda", lines)
        self.assertIn("object 1 section host_unwind", lines)
        self.assertIn("object 1 reloc_call _Unwind_Resume", lines)
        self.assertIn("object 1 reloc_ref __gxx_personality_v0", lines)
        self.assertIn("object 1 reloc_section_ref text", lines)
        self.assertIn("object 1 lsda call_site_has_cleanup", lines)
        self.assertNotIn("object 1 lsda type_table", lines)

    def test_plan_selects_objects_and_reports_private_symbols(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-host-eh-plan.") as temp_dir:
            temp = Path(temp_dir)
            one = temp / "one.o"
            two = temp / "two.o"
            one.write_bytes(b"\x7fELF" + b"\0" * 64)
            two.write_bytes(b"\x7fELF" + b"\0" * 64)
            plan = temp / "facts.plan"
            plan.write_text("object 2\n")

            write_executable(
                temp / "nm",
                textwrap.dedent(
                    """\
                    #!/usr/bin/env python3
                    import pathlib
                    import sys
                    obj = pathlib.Path(sys.argv[-1]).name
                    if obj == "one.o":
                        sys.stdout.write("                 U __cxa_throw\\n")
                    else:
                        sys.stdout.write("                 U _Unwind_Resume\\n")
                        sys.stdout.write("0000000000000000 T cppgm_eh_private_probe\\n")
                    """
                ),
            )
            write_executable(
                temp / "readelf",
                textwrap.dedent(
                    """\
                    #!/usr/bin/env python3
                    import sys
                    if sys.argv[1:2] == ["-SW"]:
                        sys.stdout.write("  [ 1] .text PROGBITS 0000000000000000 000040\\n")
                    """
                ),
            )

            env = os.environ.copy()
            env["PATH"] = f"{temp}:{env.get('PATH', '')}"
            result = subprocess.run(
                ["perl", str(SCRIPT), "--plan", str(plan), str(one), str(two)],
                cwd=REPO_ROOT,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        lines = set(result.stdout.splitlines())
        self.assertFalse(any(line.startswith("object 1 ") for line in lines))
        self.assertIn("object 2 undef _Unwind_Resume", lines)
        self.assertIn("object 2 private_symbol cppgm_eh_private_probe", lines)


if __name__ == "__main__":
    unittest.main()
