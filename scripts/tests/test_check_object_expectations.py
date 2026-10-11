#!/usr/bin/env python3

import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPO_ROOT / "scripts" / "check_object_expectations.pl"


def write_executable(path: Path, contents: str) -> None:
    path.write_text(contents)
    path.chmod(0o755)


class CheckObjectExpectationsTests(unittest.TestCase):
    def run_helper(self, platform: str, spec_text: str, *, nm_output: str,
                   nm_u_output: str = "", otool_output: str = "",
                   readelf_output: str = "") -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="cppgm-check-object.") as temp_dir:
            temp = Path(temp_dir)
            obj = temp / "probe.o"
            obj.write_bytes(b"")
            spec = temp / "probe.expect"
            spec.write_text(spec_text)

            write_executable(
                temp / "uname",
                "#!/usr/bin/env bash\n"
                f"echo {platform}\n",
            )
            write_executable(
                temp / "nm",
                textwrap.dedent(
                    f"""\
                    #!/usr/bin/env python3
                    import sys
                    if len(sys.argv) > 1 and sys.argv[1] == "-u":
                        sys.stdout.write({nm_u_output!r})
                    else:
                        sys.stdout.write({nm_output!r})
                    """
                ),
            )
            write_executable(
                temp / "otool",
                textwrap.dedent(
                    f"""\
                    #!/usr/bin/env python3
                    import sys
                    sys.stdout.write({otool_output!r})
                    """
                ),
            )
            write_executable(
                temp / "readelf",
                textwrap.dedent(
                    f"""\
                    #!/usr/bin/env python3
                    import sys
                    sys.stdout.write({readelf_output!r})
                    """
                ),
            )

            env = os.environ.copy()
            env["PATH"] = f"{temp}:{env.get('PATH', '')}"
            return subprocess.run(
                ["perl", str(SCRIPT), str(spec), str(obj)],
                cwd=REPO_ROOT,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

    def test_darwin_canonical_symbols_and_relocations(self):
        result = self.run_helper(
            "Darwin",
            textwrap.dedent(
                """\
                defined_symbol_canonical 1 _Z15mixed_subst_oneRKN7nsrepro7ProgramERKNSt3__112basic_stringIcNS3_11char_traitsIcEENS3_9allocatorIcEEEE
                absent_defined_symbol_canonical 1 _Z15mixed_subst_oneRKN7nsrepro7ProgramERKNSt3__112basic_stringIcNS4_11char_traitsIcEENS4_9allocatorIcEEEE
                relocation_class_canonical 1 data_pcrel _Z1g
                relocation_class_canonical 1 imported_data_got alias
                absent_relocation_class_canonical 1 branch_call _Z1g
                """
            ),
            nm_output=(
                "0000000000000000 T "
                "__Z15mixed_subst_oneRKN7nsrepro7ProgramERKNSt3__112basic_stringIcNS3_11char_traitsIcEENS3_9allocatorIcEEEE\n"
            ),
            otool_output=(
                "0000000000000011 X86_64_RELOC_SIGNED False __Z1g\n"
                "0000000000000033 X86_64_RELOC_GOT_LD False _alias\n"
                "0000000000000056 X86_64_RELOC_BRANCH False __Z1fv\n"
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("defined_symbol_canonical 1 _Z15mixed_subst_one", result.stdout)
        self.assertIn("relocation_class_canonical 1 data_pcrel _Z1g 1", result.stdout)
        self.assertIn("relocation_class_canonical 1 imported_data_got alias 1", result.stdout)

    def test_linux_canonical_symbols_and_relocations(self):
        result = self.run_helper(
            "Linux",
            textwrap.dedent(
                """\
                defined_symbol_canonical 1 _ZN12AbiTagBufferC1B9nqe220100ERKS_
                undefined_symbol_canonical 1 _ZN12AbiTagBuffer8setstateB9nqe220100Ej
                relocation_class_canonical 1 imported_data_got _Z1g
                absent_relocation_class_canonical 1 data_pcrel _Z1g
                """
            ),
            nm_output="0000000000000000 T _ZN12AbiTagBufferC1B9nqe220100ERKS_\n",
            nm_u_output="                 U _ZN12AbiTagBuffer8setstateB9nqe220100Ej\n",
            readelf_output=(
                "Relocation section '.rela.text' at offset 0x108 contains 2 entries:\n"
                "    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend\n"
                "0000000000000011  0000000b00000009 R_X86_64_GOTPCREL      0000000000000000 _Z1g - 4\n"
                "0000000000000056  0000000800000004 R_X86_64_PLT32         0000000000000000 _Z6read_gv - 4\n"
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("defined_symbol_canonical 1 _ZN12AbiTagBufferC1B9nqe220100ERKS_ 1", result.stdout)
        self.assertIn("undefined_symbol_canonical 1 _ZN12AbiTagBuffer8setstateB9nqe220100Ej 1", result.stdout)
        self.assertIn("relocation_class_canonical 1 imported_data_got _Z1g 1", result.stdout)

    def test_plain_c_entrypoint_is_platform_canonicalized(self):
        linux = self.run_helper(
            "Linux",
            "defined_symbol_canonical 1 _main\n",
            nm_output="0000000000000000 T main\n",
        )
        self.assertEqual(linux.returncode, 0, linux.stderr)
        self.assertIn("defined_symbol_canonical 1 _main 1", linux.stdout)

        darwin = self.run_helper(
            "Darwin",
            "defined_symbol_canonical 1 _main\n",
            nm_output="0000000000000000 T _main\n",
        )
        self.assertEqual(darwin.returncode, 0, darwin.stderr)
        self.assertIn("defined_symbol_canonical 1 _main 1", darwin.stdout)

    def test_thread_local_import_surface_is_canonicalized(self):
        result = self.run_helper(
            "Linux",
            "thread_local_import_surface_canonical 1 _ZTW4extv _Z4extv extv\n",
            nm_output="0000000000000000 W _ZTW4extv\n",
            nm_u_output="                 U extv\n",
            readelf_output=(
                "Symbol table '.symtab' contains 3 entries:\n"
                "   Num:    Value          Size Type    Bind   Vis      Ndx Name\n"
                "     1: 0000000000000000     0 FUNC    WEAK   HIDDEN     1 _ZTW4extv\n"
                "     2: 0000000000000000     0 TLS     GLOBAL DEFAULT  UND extv\n"
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "thread_local_import_surface_canonical 1 _ZTW4extv _Z4extv extv 1",
            result.stdout,
        )

    def test_thread_local_export_surface_is_canonicalized(self):
        result = self.run_helper(
            "Linux",
            "thread_local_export_surface_canonical 1 _ZTW4extv _Z4extv extv\n",
            nm_output=(
                "0000000000000000 T _ZTW4extv\n"
                "0000000000000000 D extv\n"
            ),
            readelf_output=(
                "There are 6 section headers, starting at offset 0x200:\n"
                "  [ 1] .text             PROGBITS        0000000000000000 000040 000011 00  AX  0   0 16\n"
                "  [ 2] .tdata            PROGBITS        0000000000000000 000060 000004 00 WAT  0   0  4\n"
                "Symbol table '.symtab' contains 3 entries:\n"
                "   Num:    Value          Size Type    Bind   Vis      Ndx Name\n"
                "     1: 0000000000000000    17 FUNC    GLOBAL DEFAULT    1 _ZTW4extv\n"
                "     2: 0000000000000000     4 TLS     GLOBAL DEFAULT    2 extv\n"
                "Relocation section '.rela.text' at offset 0x108 contains 1 entries:\n"
                "    Offset             Info             Type               Symbol's Value  Symbol's Name + Addend\n"
                "000000000000000c  0000000200000017 R_X86_64_TPOFF32      0000000000000000 extv + 0\n"
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "thread_local_export_surface_canonical 1 _ZTW4extv _Z4extv extv 1",
            result.stdout,
        )

    def test_host_unwind_section_is_platform_canonicalized(self):
        darwin = self.run_helper(
            "Darwin",
            textwrap.dedent(
                """\
                host_unwind_section 1
                absent_defined_symbol_substring 1 cppgm_eh_
                """
            ),
            nm_output="0000000000000000 T __Z1fv\n",
            otool_output=(
                "Load command 0\n"
                "      sectname __text\n"
                "Load command 1\n"
                "      sectname __compact_unwind\n"
            ),
        )
        self.assertEqual(darwin.returncode, 0, darwin.stderr)
        self.assertIn("host_unwind_section 1 1", darwin.stdout)
        self.assertIn("absent_defined_symbol_substring 1 cppgm_eh_ 0", darwin.stdout)

        linux = self.run_helper(
            "Linux",
            "host_unwind_section 1\n",
            nm_output="0000000000000000 T _Z1fv\n",
            readelf_output=(
                "There are 6 section headers, starting at offset 0x200:\n"
                "  [ 1] .text             PROGBITS        0000000000000000 000040 000011 00  AX  0   0 16\n"
                "  [ 2] .eh_frame         PROGBITS        0000000000000000 000060 000004 00   A  0   0  8\n"
            ),
        )
        self.assertEqual(linux.returncode, 0, linux.stderr)
        self.assertIn("host_unwind_section 1 1", linux.stdout)

    def test_darwin_emutls_thread_local_import_surface_is_canonicalized(self):
        result = self.run_helper(
            "Darwin",
            "thread_local_import_surface_canonical 1 _ZTW4extv _Z4extv extv\n",
            nm_output="0000000000000040 T __ZTW4extv\n",
            nm_u_output=(
                "                 U ___emutls_get_address\n"
                "                 U ___emutls_v.extv\n"
            ),
            otool_output=(
                "/tmp/test.o:\n"
                "Relocation information (__TEXT,__text) 3 entries\n"
                "address  pcrel length extern type    scattered symbolnum/value\n"
                "0000000f True  long   True   BRANCH  False     __ZTW4extv\n"
                "00000047 True  long   True   GOT_LD  False     ___emutls_v.extv\n"
                "0000004f True  long   True   BRANCH  False     ___emutls_get_address\n"
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "thread_local_import_surface_canonical 1 _ZTW4extv _Z4extv extv 1",
            result.stdout,
        )

    def test_darwin_emutls_thread_local_export_surface_is_canonicalized(self):
        result = self.run_helper(
            "Darwin",
            "thread_local_export_surface_canonical 1 _ZTW4extv _Z4extv extv\n",
            nm_output=(
                "0000000000000040 T __ZTW4extv\n"
                "0000000000000058 s ___emutls_t.extv\n"
                "0000000000000060 D ___emutls_v.extv\n"
            ),
            nm_u_output="                 U ___emutls_get_address\n",
            otool_output=(
                "Load command 0\n"
                "      sectname __text\n"
                "Load command 1\n"
                "      sectname __const\n"
                "Load command 2\n"
                "      sectname __data\n"
            ),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "thread_local_export_surface_canonical 1 _ZTW4extv _Z4extv extv 1",
            result.stdout,
        )


if __name__ == "__main__":
    unittest.main()
