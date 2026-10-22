#!/usr/bin/env python3

import os
import pathlib
import subprocess
import tempfile
import textwrap
import unittest


REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPARE_RESULTS = REPO_ROOT / "scripts" / "compare_results_common.pl"


def canonicalize_machine_ir(text: str) -> str:
    result = subprocess.run(
        ["perl", str(COMPARE_RESULTS), "canonicalize-machine-ir", "-"],
        input=text,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def normalize_machine_ir(text: str) -> str:
    result = subprocess.run(
        ["perl", str(COMPARE_RESULTS), "normalize-machine-ir", "-"],
        input=text,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def run_compare(
    mode: str,
    cwd: pathlib.Path,
    tests_root: str,
    env=None,
) -> subprocess.CompletedProcess[str]:
    run_env = os.environ.copy()
    if env:
        run_env.update(env)
    return subprocess.run(
        ["perl", str(COMPARE_RESULTS), mode, "ref", "my", tests_root],
        cwd=str(cwd),
        env=run_env,
        text=True,
        capture_output=True,
    )


def write_text(path: pathlib.Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class CompareResultsCommonTests(unittest.TestCase):
    def test_lowir_compare_accepts_explicit_declarations_and_readonly_globals(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "100"
            write_text(testbase.with_suffix(".t"), "extern int g;\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                declare global @ext : ptr
                declare function @helper(%arg0 : ptr) -> void

                global @table readonly = {
                  ptr addr @helper
                  ptr addr @ext
                }

                function @main() -> i64 {
                  block ^entry:
                    %0 = addr @table
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_rejects_out_of_order_reference_lowir(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "101"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }

                global @g : i64 = 0
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", output)
            self.assertIn("top-level LowIR order violation", output)

    def test_lowir_compare_accepts_out_of_order_generated_lowir(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "102"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                global @g : i64 = 0

                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }

                global @g : i64 = 0
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_direct_compare_rejects_out_of_order_generated_lowir(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "102b"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                global @g : i64 = 0

                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }

                global @g : i64 = 0
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare(
                "lowir_t",
                root,
                "tests",
                env={"CPPGM_LOWIR_DIRECT_TEXT_COMPARE": "1"},
            )
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("generated LowIR failed sanity validation", output)
            self.assertIn("top-level LowIR order violation", output)

    def test_lowir_compare_accepts_reordered_function_definitions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "102c"
            write_text(testbase.with_suffix(".t"), "int helper(); int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @helper() -> i32 [binding=strong, object=_Z6helperv] {
                  block ^entry:
                    return i32 7
                }

                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    %0 = call i32 @helper()
                    return i32 %0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    %0 = call i32 @helper()
                    return i32 %0
                }

                function @helper() -> i32 [binding=strong, object=_Z6helperv] {
                  block ^entry:
                    return i32 7
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_failure_writes_canonical_order_diff(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "102c2"
            write_text(testbase.with_suffix(".t"), "int helper(); int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @helper() -> i32 [binding=strong, object=_Z6helperv] {
                  block ^entry:
                    return i32 7
                }

                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    %0 = call i32 @helper()
                    return i32 %0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    %0 = call i32 @helper()
                    return i32 %0
                }

                function @helper() -> i32 [binding=strong, object=_Z6helperv] {
                  block ^entry:
                    return i32 8
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("canonical diff written", output)
            self.assertIn("canonicalized LowIR used for relaxed comparison", output)
            ref_compare = pathlib.Path(str(testbase) + ".my.lowir.ref.compare")
            my_compare = pathlib.Path(str(testbase) + ".my.lowir.my.compare")
            compare_diff = pathlib.Path(str(testbase) + ".my.lowir.compare.diff")
            self.assertTrue(ref_compare.exists())
            self.assertTrue(my_compare.exists())
            self.assertTrue(compare_diff.exists())
            diff_text = compare_diff.read_text()
            self.assertIn("return i32 7", diff_text)
            self.assertIn("return i32 8", diff_text)
            self.assertNotIn("@helper", ref_compare.read_text())
            self.assertNotIn("@main", my_compare.read_text())

    def test_lowir_compare_rejects_reordered_structured_global_data(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "102c3"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                global @table [storage=readonly] = {
                  i64 1
                  i64 2
                }

                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    return i32 0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    return i32 0
                }

                global @table [storage=readonly] = {
                  i64 2
                  i64 1
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("canonical diff written", output)
            compare_diff = pathlib.Path(str(testbase) + ".my.lowir.compare.diff")
            self.assertTrue(compare_diff.exists())
            diff_text = compare_diff.read_text()
            self.assertIn("i64 1", diff_text)
            self.assertIn("i64 2", diff_text)

    def test_lowir_compare_same_name_pairing_requires_signature_match(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "102e"
            write_text(testbase.with_suffix(".t"), "template overload names;\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @main() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @bind__ov3(7)
                    return i32 %0
                }

                function @bind__ov3(%f : i32) -> i32 [binding=weak] {
                  block ^entry:
                    %0 = call i32 @bind(%f)
                    return i32 %0
                }

                function @bind(%f : ptr) -> i32 [binding=weak] {
                  block ^entry:
                    return i32 1
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @main() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @bind(7)
                    return i32 %0
                }

                function @bind(%f : i32) -> i32 [binding=weak] {
                  block ^entry:
                    %0 = call i32 @bind__t2(%f)
                    return i32 %0
                }

                function @bind__t2(%f : ptr) -> i32 [binding=weak] {
                  block ^entry:
                    return i32 1
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_rejects_move_constructor_before_copy_constructor(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "103"
            write_text(testbase.with_suffix(".t"), "struct Box; int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @Box_move(%this : ptr, %other : ptr [pass=reference]) -> void [object=_ZN3BoxC1EOS_] {
                  block ^entry:
                    return
                }

                function @Box_copy(%this : ptr, %other : ptr [pass=reference]) -> void [object=_ZN3BoxC1ERKS_] {
                  block ^entry:
                    return
                }

                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("special member LowIR order violation", output)

    def test_lowir_compare_rejects_destructor_complete_before_deleting_entry(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "104"
            write_text(testbase.with_suffix(".t"), "struct Box; int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @Box_dtor_complete(%this : ptr) -> void [object=_ZN3BoxD1Ev] {
                  block ^entry:
                    return
                }

                function @Box_dtor_deleting(%this : ptr) -> void [object=_ZN3BoxD0Ev] {
                  block ^entry:
                    return
                }

                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("special member LowIR order violation", output)

    def test_lowir_compare_rejects_vtable_deleting_destructor_before_complete_slot(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "105"
            write_text(testbase.with_suffix(".t"), "struct Box; int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                global @rtti : i64 = zero
                global @Box__vtable [storage=readonly] = {
                  i64 0
                  ptr addr @rtti
                  ptr addr @Box_dtor_deleting
                  ptr addr @Box_dtor_complete
                }

                function @Box_dtor_deleting(%this : ptr) -> void [object=_ZN3BoxD0Ev] {
                  block ^entry:
                    return void
                }

                function @Box_dtor_complete(%this : ptr) -> void [object=_ZN3BoxD1Ev] {
                  block ^entry:
                    return void
                }

                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("vtable destructor slot order violation", output)

    def test_lowir_compare_rejects_init_after_fini(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "106"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }

                function @__cppgm_fini() -> void [role=fini, binding=internal] {
                  block ^entry:
                    return void
                }

                function @__cppgm_init() -> void [role=init, binding=internal] {
                  block ^entry:
                    return void
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("LowIR role order violation", output)

    def test_lowir_compare_rejects_missing_explicit_declaration(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "110"
            write_text(testbase.with_suffix(".t"), "extern int g;\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main() -> i64 {
                  block ^entry:
                    %0 = addr @missing
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_accepts_explicit_object_export_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "125"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                declare global @__external_rtti__int : ptr [object=_ZTIi]

                function @main() -> i64 [binding=strong, object=main, keep_alias=yes] {
                  block ^entry:
                    %0 = addr @__external_rtti__int
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_ignores_late_export_names_and_compares_facts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "126"
            write_text(testbase.with_suffix(".t"), "int f(); int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                declare global @extern_ro : ptr [storage=readonly, binding=strong, object=_ZTIi]
                declare function @runtime_abort() -> void [effects=readnone, unwind=no, return=noreturn, linkage=c, binding=strong, object=abort]

                global @table [storage=readonly, binding=weak, object=_ZL5table] = {
                  ptr addr @runtime_abort
                }

                function @f(%p : ptr [capture=nocapture, access=read, alias=noalias]) -> i32 [unwind=no, binding=strong, object=_Z1fPi] {
                  block ^entry:
                    %0 = index i32 [projection=array_element] %p, 0
                    return i32 1
                }

                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    %0 = addr @extern_ro
                    %1 = call i32 @f(%0)
                    return i32 %1
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                declare global @extern_ro : ptr
                declare function @rt0() -> void

                global @table = {
                  ptr addr @rt0
                }

                function @lowered_0(%p : ptr) -> i32 {
                  block ^entry:
                    %0 = index i32 %p, 0
                    return i32 1
                }

                function @entry_impl() -> i32 [role=entry] {
                  block ^entry:
                    %0 = addr @extern_ro
                    %1 = call i32 @lowered_0(%0)
                    return i32 %1
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            # The source-to-LowIR lanes compare the facts the C++ lowering
            # states; PA32's optimized lanes leave them uncompared.
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            result = run_compare("lowir_optimized_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            with_facts = (
                my_lowir
                .replace("declare global @extern_ro : ptr\n",
                         "declare global @extern_ro : ptr [storage=readonly]\n")
                .replace("declare function @rt0() -> void\n",
                         "declare function @rt0() -> void [return=noreturn, unwind=no, effects=readnone]\n")
                .replace("global @table = {", "global @table [storage=readonly] = {")
                .replace("function @lowered_0(%p : ptr) -> i32 {",
                         "function @lowered_0(%p : ptr [alias=noalias]) -> i32 [unwind=no] {")
            )
            write_text(testbase.with_suffix(".my"), with_facts)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_direct_text_compare_env_disables_relaxed_compare(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "126b"
            write_text(testbase.with_suffix(".t"), "int f(); int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @_Z1fv() -> i32 [binding=strong, object=_Z1fv] {
                  block ^entry:
                    return i32 1
                }

                function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
                  block ^entry:
                    %0 = call i32 @_Z1fv()
                    return i32 %0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @lowered_f() -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @entry_impl() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @lowered_f()
                    return i32 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            relaxed_result = run_compare("lowir_t", root, "tests")
            self.assertEqual(relaxed_result.returncode, 0, relaxed_result.stdout + relaxed_result.stderr)
            direct_result = run_compare(
                "lowir_t",
                root,
                "tests",
                env={"CPPGM_LOWIR_DIRECT_TEXT_COMPARE": "1"},
            )
            self.assertNotEqual(direct_result.returncode, 0)
            self.assertIn("direct text compare", direct_result.stdout + direct_result.stderr)

    def test_lowir_compare_still_requires_call_boundary_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "127"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @take_ref(%p : ptr [pass=reference]) -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @main() -> i32 [role=entry] {
                  block ^entry:
                    return i32 0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @take_ref(%p : ptr) -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @main() -> i32 [role=entry] {
                  block ^entry:
                    return i32 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("does not match reference after relaxed metadata", result.stdout + result.stderr)

    def test_lowir_compare_still_requires_same_function_identity_graph(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "128"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @f() -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @g() -> i32 {
                  block ^entry:
                    return i32 2
                }

                function @main() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @f()
                    return i32 %0
                }
                """
            )
            my_lowir = textwrap.dedent(
                """\
                function @x() -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @y() -> i32 {
                  block ^entry:
                    return i32 2
                }

                function @entry() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @y()
                    return i32 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("does not match reference after relaxed metadata", result.stdout + result.stderr)

    def test_lowir_compare_pairs_reordered_functions_by_structure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa18"
            testbase = root / "tests" / "200"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            ref_lowir = textwrap.dedent(
                """\
                function @f() -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @g() -> i32 {
                  block ^entry:
                    return i32 2
                }

                function @main() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @f()
                    return i32 %0
                }
                """
            )
            # Same program: the two helpers are emitted in the opposite order and
            # carry different, unmatchable symbol names, but main still calls the
            # return-1 helper. Structural pairing must line them up by shape so the
            # relaxed compare accepts this instead of mispairing by emission order.
            my_lowir = textwrap.dedent(
                """\
                function @y() -> i32 {
                  block ^entry:
                    return i32 2
                }

                function @x() -> i32 {
                  block ^entry:
                    return i32 1
                }

                function @main() -> i32 [role=entry] {
                  block ^entry:
                    %0 = call i32 @x()
                    return i32 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), ref_lowir)
            write_text(testbase.with_suffix(".my"), my_lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_rejects_invalid_role_owner(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "130"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                declare global @bad : ptr [role=entry]

                function @main() -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_accepts_direct_object_return_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "145"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                global @out = {
                  zero 16
                }

                function @make_pair(%x : i64) -> obj<16x8> {
                  slot $tmp : obj<16x8>
                  block ^entry:
                    %0 = addr $tmp
                    store i64 %x, %0
                    return obj<16x8> $tmp
                }

                function @main() -> i64 {
                  block ^entry:
                    %0 = call obj<16x8> @make_pair(7)
                    %1 = addr @out
                    copyobj 16x8 %0, %1
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_accepts_indirect_call_signature_returning_object(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "146"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%fp : ptr) -> i64 {
                  block ^entry:
                    %0 = call obj<8x8> %fp() as () -> obj<8x8>
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_rejects_invalid_parameter_pass_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "150"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%x : i64 [pass=reference]) -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_rejects_invalid_parameter_capture_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "156"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%x : i64 [capture=nocapture]) -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_rejects_invalid_parameter_access_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "158"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%x : i64 [access=read]) -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_rejects_invalid_parameter_alias_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "159b"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%x : i64 [alias=noalias]) -> i64 {
                  block ^entry:
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_accepts_variadic_function_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "160"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @sum(%count : i64) -> i64 [arity=variadic] {
                  block ^entry:
                    return i64 %count
                }

                function @main() -> i64 {
                  block ^entry:
                    %0 = call i64 @sum(7, 11, 13)
                    return i64 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_accepts_switch_terminator(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "162"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main() -> i64 {
                  block ^entry:
                    %which = const i64 1
                    switch %which, ^miss, 0:^zero, 1:^one
                  block ^zero:
                    return i64 5
                  block ^one:
                    return i64 7
                  block ^miss:
                    return i64 9
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_accepts_global_storage_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "166"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                global @ro_value : i64 [storage=readonly, binding=weak] = 7

                function @main() -> i64 {
                  block ^entry:
                    %0 = load i64 @ro_value
                    return i64 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_accepts_alignment_aware_storage_ops(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "167"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%src : obj<8x4>, %dst : ptr) -> i64 {
                  block ^entry:
                    copyobj 8x4 %src, %dst
                    zeroinit 8x4 %dst
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_tracks_object_load_types(self) -> None:
        lowir = textwrap.dedent(
            """\
            function @main() -> i64 [role=entry] {
              slot $source : obj<8x8>
              slot $destination : obj<8x8>
              block ^entry:
                store i64 7, $source
                %object = load obj<8x8> $source
                %out = addr $destination
                copyobj 8x8 %object, %out
                %value = load i64 %out
                %bad = cmp ne i64 %value, 7
                return i64 %bad
            }
            """
        )
        cases = {
            "object-load": (lowir, True),
            "size-mismatch": (lowir.replace("copyobj 8x8", "copyobj 16x8"), False),
            "alignment-mismatch": (lowir.replace("copyobj 8x8", "copyobj 8x4"), False),
            "undefined-storage": (lowir.replace("obj<8x8> $source\n", "obj<8x8> $missing\n"), False),
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for name, (generated, accepted) in cases.items():
                with self.subTest(case=name):
                    testbase = root / name / "case"
                    for suffix, text in {
                        ".t": lowir,
                        ".ref": lowir,
                        ".my": generated,
                        ".ref.exit_status": "EXIT_SUCCESS\n",
                        ".my.exit_status": "EXIT_SUCCESS\n",
                    }.items():
                        write_text(testbase.with_suffix(suffix), text)
                    result = run_compare("lowir_optimized_t", root, name)
                    output = result.stdout + result.stderr
                    self.assertEqual(result.returncode, 0 if accepted else 1, output)
                    if not accepted:
                        self.assertIn("generated LowIR failed sanity validation", output)

    def test_lowir_compare_rejects_bad_storage_op_alignment(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "168"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @main(%src : obj<8x4>, %dst : ptr) -> i64 {
                  block ^entry:
                    copyobj 8x3 %src, %dst
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_rejects_fixed_arity_call_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "170"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @sum(%count : i64) -> i64 {
                  block ^entry:
                    return i64 %count
                }

                function @main() -> i64 {
                  block ^entry:
                    %0 = call i64 @sum(7, 11)
                    return i64 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("expects exactly 1 argument(s), got 2", result.stdout + result.stderr)

    def test_lowir_compare_rejects_linkage_metadata_on_call_signature(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "175"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @helper(%x : i64) -> i64 {
                  block ^entry:
                    return i64 %x
                }

                function @main() -> i64 {
                  block ^entry:
                    %fp = addr @helper
                    %0 = call i64 %fp(7) as (%arg0 : i64) -> i64 [linkage=c]
                    return i64 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_rejects_trivial_lifecycle_metadata_on_call_signature(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "175b"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @callee(%this : ptr) -> void {
                  block ^entry:
                    return void
                }

                function @main() -> i64 {
                  block ^entry:
                    %this = addr @callee
                    call void @callee(%this) as (%arg0 : ptr) -> void [trivial_lifecycle=yes]
                    return i64 0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("invalid reference LowIR", result.stdout + result.stderr)

    def test_lowir_compare_accepts_indirect_call_signature_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "180"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @helper(%x : i64) -> i64 {
                  block ^entry:
                    return i64 %x
                }

                function @main() -> i64 {
                  block ^entry:
                    %fp = addr @helper
                    %0 = call i64 %fp(7) as (%arg0 : i64) -> i64
                    return i64 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_compare_rejects_missing_indirect_call_signature_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa14"
            testbase = root / "tests" / "190"
            write_text(testbase.with_suffix(".t"), "int main();\n")
            write_text(testbase.with_suffix(".ref.exit_status"), "EXIT_SUCCESS\n")
            write_text(testbase.with_suffix(".my.exit_status"), "EXIT_SUCCESS\n")
            lowir = textwrap.dedent(
                """\
                function @helper(%x : i64) -> i64 {
                  block ^entry:
                    return i64 %x
                }

                function @main() -> i64 {
                  block ^entry:
                    %fp = addr @helper
                    %0 = call i64 %fp(7)
                    return i64 %0
                }
                """
            )
            write_text(testbase.with_suffix(".ref"), lowir)
            write_text(testbase.with_suffix(".my"), lowir)
            result = run_compare("lowir_t", root, "tests")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("indirect call requires explicit call signature", result.stdout + result.stderr)

    def test_program_compare_reports_implementation_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa8"
            testbase = root / "tests" / "100"
            write_text(pathlib.Path(str(testbase) + ".t"), "caller\n")
            write_text(pathlib.Path(str(testbase) + ".ref.impl.exit_status"), "0\n")
            write_text(pathlib.Path(str(testbase) + ".my.impl.exit_status"), "124\n")
            result = run_compare("program_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("implementation timed out", output)

    def test_program_compare_reports_implementation_oom(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa8"
            testbase = root / "tests" / "105"
            write_text(pathlib.Path(str(testbase) + ".t"), "caller\n")
            write_text(pathlib.Path(str(testbase) + ".ref.impl.exit_status"), "0\n")
            write_text(pathlib.Path(str(testbase) + ".my.impl.exit_status"), "125\n")
            result = run_compare("program_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("implementation ran out of memory", output)

    def test_program_compare_reports_program_timeout_before_stdout_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa8"
            testbase = root / "tests" / "110"
            write_text(pathlib.Path(str(testbase) + ".t"), "caller\n")
            write_text(pathlib.Path(str(testbase) + ".ref.impl.exit_status"), "0\n")
            write_text(pathlib.Path(str(testbase) + ".my.impl.exit_status"), "0\n")
            write_text(pathlib.Path(str(testbase) + ".ref.program.exit_status"), "0\n")
            write_text(pathlib.Path(str(testbase) + ".my.program.exit_status"), "124\n")
            write_text(pathlib.Path(str(testbase) + ".ref.program.stdout"), "finished\n")
            write_text(pathlib.Path(str(testbase) + ".my.program.stdout"), "")
            result = run_compare("program_t", root, "tests")
            output = result.stdout + result.stderr
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("generated program timed out", output)
            self.assertNotIn("exit status and stdout do not match", output)

    def test_normalize_machine_ir_only_normalizes_header(self) -> None:
        raw = textwrap.dedent(
            """\
            machine_ir x86_64 linux

            function @f
              block ^entry
                load.i64 r8, [rbp-8]
            """
        )
        expected = textwrap.dedent(
            """\
            machine_ir x86_64 <host-target>

            function @f
              block ^entry
                load.i64 r8, [rbp-8]
            """
        )
        self.assertEqual(normalize_machine_ir(raw), expected)

    def test_canonicalize_machine_ir_normalizes_header_and_frame_offsets(self) -> None:
        raw = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              frame
                param-slot %x -> [rbp-8] : i64

              block ^entry
                load.i64 r8, [rbp-8]
                store.i64 [rbp+24], r8
            """
        )
        expected = textwrap.dedent(
            """\
            machine_ir x86_64 <host-target>

            function @f
              frame
                param-slot %x -> [rbp-<off0>] : i64

              block ^entry
                load.i64 r8, [rbp-<off0>]
                store.i64 [rbp+<off0>], r8
            """
        )
        self.assertEqual(canonicalize_machine_ir(raw), expected)

    def test_canonicalize_machine_ir_reuses_same_placeholder_per_bucket(self) -> None:
        raw = textwrap.dedent(
            """\
            machine_ir x86_64 linux

            function @f
              block ^entry
                load.i64 r8, [rbp-8]
                load.i64 r9, [rbp-8]
                load.i64 r10, [rbp-24]
                store.i64 [rbp+16], r8
                store.i64 [rbp+16], r9
            """
        )
        expected = textwrap.dedent(
            """\
            machine_ir x86_64 <host-target>

            function @f
              block ^entry
                load.i64 r8, [rbp-<off0>]
                load.i64 r9, [rbp-<off0>]
                load.i64 <gpr0>, [rbp-<off1>]
                store.i64 [rbp+<off0>], r8
                store.i64 [rbp+<off0>], r9
            """
        )
        self.assertEqual(canonicalize_machine_ir(raw), expected)

    def test_canonicalize_machine_ir_collapses_allocator_only_differences(self) -> None:
        left = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              block ^entry
                mov rbx, 1
                add rbx, r10
                fmov.f64 xmm2, xmm3
                ret rax
            """
        )
        right = textwrap.dedent(
            """\
            machine_ir x86_64 linux

            function @f
              block ^entry
                mov r12, 1
                add r12, r13
                fmov.f64 xmm4, xmm5
                ret rax
            """
        )
        self.assertEqual(canonicalize_machine_ir(left), canonicalize_machine_ir(right))

    def test_canonicalize_machine_ir_normalizes_free_gpr_choices(self) -> None:
        raw = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              frame
                preserve r12
                preserve rbx

              block ^entry
                mov r12, rbx
                add r12, r13
                mov rax, r12
                ret rax
            """
        )
        expected = textwrap.dedent(
            """\
            machine_ir x86_64 <host-target>

            function @f
              frame
                preserve <gpr0>
                preserve <gpr1>

              block ^entry
                mov <gpr0>, <gpr1>
                add <gpr0>, <gpr2>
                mov rax, <gpr0>
                ret rax
            """
        )
        self.assertEqual(canonicalize_machine_ir(raw), expected)

    def test_canonicalize_machine_ir_normalizes_free_xmm_choices(self) -> None:
        raw = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              block ^entry
                fadd.f32 xmm2, xmm0, xmm3
                fmul.f64 xmm4, xmm2, xmm5
                fmov.f64 xmm1, xmm4
                ret rax
            """
        )
        expected = textwrap.dedent(
            """\
            machine_ir x86_64 <host-target>

            function @f
              block ^entry
                fadd.f32 <xmm0>, xmm0, <xmm1>
                fmul.f64 <xmm2>, <xmm0>, <xmm3>
                fmov.f64 xmm1, <xmm2>
                ret rax
            """
        )
        self.assertEqual(canonicalize_machine_ir(raw), expected)

    def test_canonicalize_machine_ir_preserves_compare_branch_shape(self) -> None:
        direct = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              block ^entry
                cmp.i32 rax, rdx
                jne ^bad
                jmp ^ok
            """
        )
        materialized = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              block ^entry
                cmp.i32 rax, rdx
                sete r8
                movzx r8, r8
                cmp.i64 r8, 0
                jne ^bad
                jmp ^ok
            """
        )
        self.assertNotEqual(canonicalize_machine_ir(direct), canonicalize_machine_ir(materialized))

    def test_canonicalize_machine_ir_preserves_call_shape_and_width(self) -> None:
        direct = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              block ^entry
                call @g
                cmp.i32 rax, rdx
            """
        )
        indirect_and_wide = textwrap.dedent(
            """\
            machine_ir x86_64 macos

            function @f
              block ^entry
                call [rax]
                cmp.i64 rax, rdx
            """
        )
        self.assertNotEqual(canonicalize_machine_ir(direct), canonicalize_machine_ir(indirect_and_wide))

    def test_mir_structural_mode_requires_checked_in_cmir(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa18"
            testbase = root / "tests" / "structural" / "100"
            write_text(testbase.with_suffix(".t"), "noop\n")
            write_text(testbase.with_suffix(".ref.impl.exit_status"), "0\n")
            write_text(testbase.with_suffix(".my.impl.exit_status"), "0\n")
            write_text(testbase.with_suffix(".ref.mir"), "machine_ir x86_64 macos\n")
            write_text(testbase.with_suffix(".my.mir"), "machine_ir x86_64 linux\n")
            write_text(testbase.with_suffix(".ref.program.exit_status"), "0\n")
            write_text(testbase.with_suffix(".my.program.exit_status"), "0\n")
            write_text(testbase.with_suffix(".ref.program.stdout"), "")
            write_text(testbase.with_suffix(".my.program.stdout"), "")
            result = run_compare("mir_structural_t", root, "tests/structural")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing structural machine IR oracle", result.stdout + result.stderr)

    def test_mir_mode_without_checked_in_mir_compares_program_only(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa18"
            testbase = root / "tests" / "behavior" / "100"
            write_text(testbase.with_suffix(".t"), "noop\n")
            write_text(testbase.with_suffix(".ref.impl.exit_status"), "0\n")
            write_text(testbase.with_suffix(".my.impl.exit_status"), "0\n")
            write_text(
                testbase.with_suffix(".my.mir"),
                textwrap.dedent(
                    """\
                    machine_ir x86_64 linux

                    function @f
                      block ^entry
                        cmp.i32 rax, rdx
                    """
                ),
            )
            write_text(testbase.with_suffix(".ref.program.exit_status"), "0\n")
            write_text(testbase.with_suffix(".my.program.exit_status"), "0\n")
            write_text(testbase.with_suffix(".ref.program.stdout"), "")
            write_text(testbase.with_suffix(".my.program.stdout"), "")
            result = run_compare("mir_t", root, "tests/behavior")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_mir_structural_failure_writes_debug_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp) / "pa18"
            testbase = root / "tests" / "structural" / "420"
            write_text(testbase.with_suffix(".t"), "noop\n")
            write_text(testbase.with_suffix(".ref.impl.exit_status"), "0\n")
            write_text(testbase.with_suffix(".my.impl.exit_status"), "0\n")
            ref_raw = textwrap.dedent(
                """\
                machine_ir x86_64 macos

                function @f
                  block ^entry
                    cmp.i32 rax, rdx
                    jne ^bad
                """
            )
            write_text(testbase.with_suffix(".ref.mir"), ref_raw)
            write_text(
                testbase.with_suffix(".ref.cmir"),
                canonicalize_machine_ir(ref_raw),
            )
            write_text(
                testbase.with_suffix(".my.mir"),
                textwrap.dedent(
                    """\
                    machine_ir x86_64 linux

                    function @f
                      block ^entry
                        cmp.i32 rax, rdx
                        sete r8
                        movzx r8, r8
                        cmp.i64 r8, 0
                        jne ^bad
                    """
                ),
            )
            write_text(testbase.with_suffix(".ref.program.exit_status"), "0\n")
            write_text(testbase.with_suffix(".my.program.exit_status"), "0\n")
            write_text(testbase.with_suffix(".ref.program.stdout"), "")
            write_text(testbase.with_suffix(".my.program.stdout"), "")
            result = run_compare("mir_structural_t", root, "tests/structural")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("structural machine IR dumps do not match", result.stdout + result.stderr)
            self.assertTrue(testbase.with_suffix(".my.cmir").exists())

if __name__ == "__main__":
    unittest.main()
