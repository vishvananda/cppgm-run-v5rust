#!/usr/bin/env python3

from pathlib import Path
import os
import re
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
COMPARE = REPO_ROOT / "scripts" / "compare_results_common.pl"


REFERENCE = """function @main(%left : i64, %right : i64) -> i64 {
  slot $saved : i64

  block ^entry:
    %sum = binary add i64 %left, %right
    store i64 %sum, $saved
    jump ^done

  block ^done:
    %result = load i64 $saved
    return i64 %result
}
"""


RENAMED = """function @main(%a : i64, %b : i64) -> i64 {
  slot $__o1inl0__saved : i64

  block ^__o1inl0__entry:
    %__o1inl0__sum = binary add i64 %a, %b
    store i64 %__o1inl0__sum, $__o1inl0__saved
    jump ^__o1inl0__done

  block ^__o1inl0__done:
    %__o1inl0__result = load i64 $__o1inl0__saved
    return i64 %__o1inl0__result
}
"""


class LowIrComparisonTest(unittest.TestCase):
    def compare(self, reference, generated, *, direct=False, mode="lowir_t"):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            test = root / "case.t"
            test.write_text(reference, encoding="utf-8")
            (root / "case.ref").write_text(reference, encoding="utf-8")
            (root / "case.my").write_text(generated, encoding="utf-8")
            for suffix in ("ref", "my"):
                (root / f"case.{suffix}.exit_status").write_text(
                    "EXIT_SUCCESS\n", encoding="utf-8"
                )
            environment = os.environ.copy()
            # The harness reads these from the calling make (test-report
            # exports KEEP_GOING=1, under which a failed comparison exits 0);
            # each case here states its own expectations.
            for name in (
                "KEEP_GOING",
                "CPPGM_CHECK_AUTO_KEEP_GOING",
                "CPPGM_BATCH_TESTS",
                "WRAPPED_BATCH_STDIN",
                "CPPGM_TEST_COUNTS_FILE",
                "CPPGM_LOWER_CHECK_APP",
                "CPPGM_BEHAVIOR_CHECK_APP",
                "CPPGM_LOWIR_DIRECT_TEXT_COMPARE",
            ):
                environment.pop(name, None)
            if direct:
                environment["CPPGM_LOWIR_DIRECT_TEXT_COMPARE"] = "1"
            return subprocess.run(
                [
                    "perl",
                    str(COMPARE),
                    mode,
                    "ref",
                    "my",
                    str(test),
                ],
                cwd=REPO_ROOT,
                env=environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

    def test_relaxed_compare_ignores_function_local_names(self):
        result = self.compare(REFERENCE, RENAMED)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_ignores_unused_local_slots_on_either_side(self):
        for name, storage in (("throw_alloc__2", "ptr"), ("saved_extra", "obj<16x8>")):
            with self.subTest(name=name):
                padded = REFERENCE.replace(
                    "  slot $saved : i64", f"  slot ${name} : {storage}\n  slot $saved : i64"
                )
                for reference, generated in ((padded, RENAMED), (REFERENCE, padded)):
                    result = self.compare(reference, generated)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                result = self.compare(REFERENCE, padded, direct=True)
                self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_keeps_slot_uses_local_to_each_function(self):
        unused = """function @extra() -> void {
  slot $saved : ptr
  block ^entry:
    return void
}
"""
        result = self.compare(REFERENCE + unused, RENAMED + unused.replace("  slot $saved : ptr\n", ""))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_preserves_referenced_slot_types_and_declarations(self):
        for generated in (REFERENCE.replace("slot $saved : i64", "slot $saved : ptr"),
                          REFERENCE.replace("  slot $saved : i64\n", "")):
            result = self.compare(REFERENCE, generated)
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_validates_unused_slots_before_dropping_them(self):
        duplicate = REFERENCE.replace(
            "  slot $saved : i64", "  slot $unused : ptr\n  slot $unused : ptr\n  slot $saved : i64"
        )
        result = self.compare(REFERENCE, duplicate)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("duplicate slot", result.stdout + result.stderr)

    def test_lowir_sanity_accepts_phi_loop_backedge_value(self):
        program = """function @main() -> i64 {
  block ^entry:
    jump ^loop

  block ^loop:
    %current = phi i64 [^entry: 0, ^latch: %next]
    %done = cmp ge i64 %current, 3
    branch %done, ^exit, ^latch

  block ^latch:
    %next = binary add i64 %current, 1
    jump ^loop

  block ^exit:
    return i64 %current
}
"""
        result = self.compare(program, program)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_retains_operand_identity(self):
        changed = RENAMED.replace(
            "binary add i64 %a, %b", "binary add i64 %a, %a"
        )
        result = self.compare(REFERENCE, changed)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)

    def test_relaxed_compare_retains_control_flow_identity(self):
        changed = RENAMED.replace(
            "jump ^__o1inl0__done", "jump ^__o1inl0__entry"
        )
        result = self.compare(REFERENCE, changed)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)

    def test_direct_compare_still_requires_exact_names(self):
        result = self.compare(REFERENCE, RENAMED, direct=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("direct text compare", result.stdout)

    def test_relaxed_compare_pairs_global_presentation_by_object_identity(self):
        reference = """declare global @rtti : ptr [object=_ZTI1X]

global @vtable [object=_ZTV1X] = {
  ptr addr @rtti
}

function @main() -> i64 {
  block ^entry:
    %address = addr @vtable
    return i64 0
}
"""
        generated = """declare global @__typed_rtti : ptr [role=rtti_data, object=_ZTI1X]

global @__typed_vtable [object=_ZTV1X] = {
  ptr addr @__typed_rtti
}

function @main() -> i64 {
  block ^entry:
    %address = addr @__typed_vtable
    return i64 0
}
"""
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_retains_global_initializer_shape(self):
        reference = """global @left : i64 = 1

function @main() -> i64 {
  block ^entry:
    return i64 0
}
"""
        generated = reference.replace("= 1", "= 2")
        result = self.compare(reference, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)

    def test_relaxed_compare_normalizes_trivial_lifecycle_calls(self):
        reference = """function @main() -> i64 {
  slot $value : obj<1x1>
  block ^entry:
    %address = addr $value
    call void @empty_ctor(%address)
    return i64 0
}

function @empty_ctor(%this : ptr) -> void [binding=weak, trivial_lifecycle=yes] {
  slot $saved : ptr
  block ^entry:
    store ptr %this, $saved
    return void
}
"""
        generated = """function @main() -> i64 {
  slot $value : obj<1x1>
  block ^entry:
    %address = addr $value
    return i64 0
}
"""
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_rejects_generated_trivial_lifecycle_metadata(self):
        generated = REFERENCE.replace(
            ") -> i64 {", ") -> i64 [trivial_lifecycle=yes] {", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("trivial_lifecycle= is not defined", result.stdout)

    def test_relaxed_compare_normalizes_legacy_unreachable_call(self):
        reference = """declare function @impossible() -> void [role=unreachable, effects=readnone, unwind=no, return=noreturn]

function @trap() -> void {
  block ^entry:
    call void @impossible()
    return void
}
"""
        generated = """function @trap() -> void {
  block ^entry:
    unreachable
}
"""
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_rejects_generated_unreachable_role(self):
        generated = REFERENCE.replace(
            ") -> i64 {", ") -> i64 [role=unreachable] {", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("function role unreachable is not defined", result.stdout)

    def test_lowir_sanity_rejects_removed_access_none(self):
        generated = REFERENCE.replace(
            "%left : i64", "%left : ptr [access=none]", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unknown parameter access mode 'none'", result.stdout)

    def test_lowir_sanity_rejects_removed_pass_decay(self):
        generated = REFERENCE.replace(
            "%left : i64", "%left : ptr [pass=decay]", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not defined by the LowIR format", result.stdout)

    def test_relaxed_compare_rejects_generated_capture_metadata(self):
        generated = REFERENCE.replace(
            "%left : i64", "%left : ptr [capture=nocapture]", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("capture= and access= are not defined", result.stdout)

    def test_relaxed_compare_rejects_generated_access_metadata(self):
        generated = REFERENCE.replace(
            "%left : i64", "%left : ptr [access=read]", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("capture= and access= are not defined", result.stdout)

    def test_relaxed_compare_rejects_generated_source_origin_projection(self):
        generated = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "%projected = index i8 [projection=base_subobject] nullptr, 0\n"
            "    %sum = binary add i64 %left, %right",
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("index projections base_subobject and reference_field are not defined", result.stdout)

    def test_relaxed_compare_accepts_legacy_source_origin_projection_reference(self):
        reference = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "%projected = index i8 [projection=reference_field] nullptr, 0\n"
            "    %sum = binary add i64 %left, %right",
        )
        generated = reference.replace(" [projection=reference_field]", "")
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_pairs_legacy_decay_by_ignored_shape(self):
        reference = """function @read_old(%value : ptr [pass=decay]) -> i32 {
  block ^entry:
    %loaded = load i32 %value
    return i32 %loaded
}

function @observe_old(%value : ptr [pass=decay]) -> void {
  block ^entry:
    return void
}

function @main() -> i32 [role=entry] {
  slot $value : i32

  block ^entry:
    %address = addr $value
    call void @observe_old(%address)
    %result = call i32 @read_old(%address)
    return i32 %result
}
"""
        generated = """function @observe_new(%value : ptr) -> void {
  block ^entry:
    return void
}

function @read_new(%value : ptr) -> i32 {
  block ^entry:
    %loaded = load i32 %value
    return i32 %loaded
}

function @main() -> i32 [role=entry] {
  slot $value : i32

  block ^entry:
    %address = addr $value
    call void @observe_new(%address)
    %result = call i32 @read_new(%address)
    return i32 %result
}
"""
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_sanity_rejects_removed_unary_decay(self):
        generated = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "%decayed = unary decay ptr nullptr\n"
            "    %sum = binary add i64 %left, %right",
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not defined by the LowIR format", result.stdout)

    def test_lowir_sanity_rejects_removed_reference_pass(self):
        generated = REFERENCE.replace(
            "%left : i64", "%left : ptr [pass=reference]", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("pass=reference is not defined", result.stdout)

    def test_relaxed_compare_maps_legacy_reference_to_by_address(self):
        reference = REFERENCE.replace(
            "%left : i64", "%left : ptr [pass=reference]", 1
        )
        generated = reference.replace("pass=reference", "pass=by_address")
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_erases_legacy_unary_decay(self):
        reference = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "%pointer = copy ptr nullptr\n"
            "    %decayed = unary decay ptr %pointer\n"
            "    %sum = binary add i64 %left, %right",
        )
        generated = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "%pointer = copy ptr nullptr\n"
            "    %sum = binary add i64 %left, %right",
        )
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_relaxed_compare_retains_nontrivial_calls(self):
        reference = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "call void @effect()\n    %sum = binary add i64 %left, %right",
        ).replace(
            "function @main",
            "declare function @effect() -> void\n\nfunction @main",
        )
        result = self.compare(reference, REFERENCE)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)

    def test_relaxed_compare_allows_additional_generated_object_root(self):
        generated = REFERENCE.replace(
            ") -> i64 {", ") -> i64 [object_root=yes] {", 1
        )
        result = self.compare(REFERENCE, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_requires_reference_object_root(self):
        reference = REFERENCE.replace(
            ") -> i64 {", ") -> i64 [object_root=yes] {", 1
        )
        result = self.compare(reference, REFERENCE)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)

    def test_relaxed_compare_rejects_an_inserted_call_and_its_definition(self):
        generated = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "call void @later_helper()\n    %sum = binary add i64 %left, %right",
        ) + """\nfunction @later_helper() -> void {
  block ^entry:
    return void
}
"""
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_allows_declaration_to_definition_refinement(self):
        reference = "declare function @helper() -> i64\n"
        generated = """function @helper() -> i64 {
  block ^entry:
    return i64 0
}
"""
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_normalizes_tls_wrapper_access(self):
        reference = """declare global @value : i32 [storage=thread_local]
declare function @wrapper() -> ptr [tls_for=@value]

function @main() -> i32 {
  block ^entry:
    %address = addr @value
    %result = load i32 %address
    return i32 %result
}
"""
        generated = reference.replace(
            "%address = addr @value", "%address = call ptr @wrapper()"
        )
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_normalizes_direct_tls_load(self):
        reference = """declare function @wrapper() -> ptr [tls_for=@value]
global @value : i32 [storage=thread_local] = 1

function @main() -> i32 {
  block ^entry:
    %result = load i32 @value
    return i32 %result
}
"""
        generated = reference.replace(
            "%result = load i32 @value",
            "%address = call ptr @wrapper()\n    %result = load i32 %address",
        )
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_allows_unretained_unused_declaration(self):
        reference = "declare function @unused() -> void\n\n" + REFERENCE
        result = self.compare(reference, REFERENCE)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_allows_unretained_unused_weak_body(self):
        reference = REFERENCE + """
function @unused() -> void [binding=weak] {
  block ^entry:
    return void
}
"""
        result = self.compare(reference, REFERENCE)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_requires_referenced_weak_body(self):
        reference = REFERENCE.replace(
            "%sum = binary add i64 %left, %right",
            "call void @needed()\n    %sum = binary add i64 %left, %right",
        ) + """
function @needed() -> void [binding=weak] {
  block ^entry:
    return void
}
"""
        result = self.compare(reference, REFERENCE)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_rejects_inserted_locals_and_blocks(self):
        # Every line of a body is compared: an output that holds the
        # reference's instructions plus others differs, as a leaner one does.
        generated = RENAMED.replace(
            "slot $__o1inl0__saved : i64",
            "slot $inserted : ptr\n  slot $__o1inl0__saved : i64",
        ).replace(
            "block ^__o1inl0__entry:",
            "block ^inserted:\n    jump ^__o1inl0__entry\n\n"
            "  block ^__o1inl0__entry:",
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_rejects_an_extra_instruction(self):
        generated = RENAMED.replace(
            "    store i64 %__o1inl0__sum, $__o1inl0__saved\n",
            "    store i64 %__o1inl0__sum, $__o1inl0__saved\n"
            "    %__o1inl0__again = load i64 $__o1inl0__saved\n",
        )
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_rejects_an_extra_definition(self):
        generated = RENAMED + "\nfunction @extra() -> void [binding=internal] {\n  block ^entry:\n    return void\n}\n"
        result = self.compare(REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_allows_registered_static_finalizer(self):
        reference = """declare function @destroy(%value : ptr) -> void

global @value : i32 = 0
global @guard : i64 = 0

function @get() -> ptr {
  block ^entry:
    %address = addr @value
    return ptr %address
}

function @fini() -> void [role=fini] {
  block ^entry:
    %ready = load i64 @guard
    %needed = cmp ne i64 %ready, 0
    branch %needed, ^destroy, ^done
  block ^destroy:
    %address = addr @value
    call void @destroy(%address)
    jump ^done
  block ^done:
    return void
}
"""
        generated = """declare function @destroy(%value : ptr) -> void
declare function @atexit(%callback : ptr) -> i32

global @value : i32 = 0
global @guard : i64 = 0

function @get() -> ptr {
  block ^entry:
    %registered = call i32 @atexit(@helper)
    %address = addr @value
    return ptr %address
}

function @helper() -> void {
  block ^entry:
    %address = addr @value
    call void @destroy(%address)
    return void
}
"""
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    FINALIZER_ARRAY_REFERENCE = """declare function @destroy(%value : ptr) -> void
declare function @finish(%value : ptr) -> void

global @items : obj<12x4> = {
  zero 12
}
global @other : i32 = 0

function @fini() -> void [role=fini] {
  block ^entry:
    %base = addr @items
    %third = index i8 [projection=array_element] %base, 8
    call void @destroy(%third)
    %second = index i8 [projection=array_element] %base, 4
    call void @destroy(%second)
    %first = index i8 [projection=array_element] %base, 0
    call void @destroy(%first)
    %other = addr @other
    call void @finish(%other)
    return void
}
"""

    def test_relaxed_compare_rejects_finalizer_missing_array_elements(self):
        generated = """declare function @destroy(%value : ptr) -> void
declare function @finish(%value : ptr) -> void

global @items : obj<12x4> = {
  zero 12
}
global @other : i32 = 0

function @fini() -> void [role=fini] {
  block ^entry:
    %base = addr @items
    call void @destroy(%base)
    %other = addr @other
    call void @finish(%other)
    return void
}
"""
        result = self.compare(self.FINALIZER_ARRAY_REFERENCE, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_allows_finalizer_destruction_loop(self):
        generated = """declare function @destroy(%value : ptr) -> void
declare function @finish(%value : ptr) -> void

global @items : obj<12x4> = {
  zero 12
}
global @other : i32 = 0

function @fini() -> void [role=fini] {
  slot $index : i64
  block ^entry:
    store i64 3, $index
    jump ^test
  block ^test:
    %remaining = load i64 $index
    %more = cmp ne i64 %remaining, 0
    branch %more, ^body, ^done
  block ^body:
    %next = binary sub i64 %remaining, 1
    store i64 %next, $index
    %offset = binary mul i64 %next, 4
    %base = addr @items
    %element = index i8 [projection=array_element] %base, %offset
    call void @destroy(%element)
    jump ^test
  block ^done:
    %other = addr @other
    call void @finish(%other)
    return void
}
"""
        result = self.compare(self.FINALIZER_ARRAY_REFERENCE, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_rejects_reordered_finalizer(self):
        reference = """declare function @destroy(%value : ptr) -> void

global @first : i32 = 0
global @second : i32 = 0

function @fini() -> void [role=fini] {
  block ^entry:
    %late = addr @second
    call void @destroy(%late)
    %early = addr @first
    call void @destroy(%early)
    return void
}
"""
        generated = """declare function @destroy(%value : ptr) -> void

global @first : i32 = 0
global @second : i32 = 0

function @fini() -> void [role=fini] {
  block ^entry:
    %early = addr @first
    call void @destroy(%early)
    %late = addr @second
    call void @destroy(%late)
    return void
}
"""
        result = self.compare(reference, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_relaxed_compare_rejects_an_explicit_termination_path_in_a_nothrow_function(self):
        # In a non-throwing function a cleanup handler ends in `resume`
        # (the comparison drops the resume itself); a handler that spells
        # the termination out has lines the reference does not.
        reference = """function @work() -> void [unwind=no] {
  block ^entry:
    eh_try ^cleanup
    eh_end
    return void
  block ^cleanup:
    resume
}
"""
        generated = """function @work() -> void [unwind=no] {
  slot $exception : ptr
  block ^entry:
    eh_try ^cleanup
    eh_end
    return void
  block ^cleanup:
    eh_catch_all, 1
    %exception = exception ptr
    store ptr %exception, $exception
    call void @terminate()
    return void
}

declare function @terminate() -> void
"""
        result = self.compare(reference, generated)
        self.assertNotEqual(result.returncode, 0)

    LITERALS = """declare function @sink(%a : i32, %b : i64, %c : f64) -> void

global @three : f64 = 3.0

global @half : f32 = 0.5

function @main(%x : i32, %p : ptr) -> i64 {
  slot $v : u32
  slot $q : ptr
  slot $z : i32
  block ^entry:
    store u32 4294967295, $v
    store ptr 0, $q
    store i32 0, $z
    %a = binary add i32 %x, 16
    %b = cmp eq i32 %a, -1
    %f = const f64 1.5
    %g = const f32 0.1
    %h = const f80 12.0L
    %n = cmp eq ptr %p, 0
    call void @sink(%a, 7, 1.5)
    return i64 0
}
"""

    def test_relaxed_compare_reads_literals_by_value(self):
        generated = (
            self.LITERALS
            .replace("store u32 4294967295", "store u32 -1")
            .replace("store ptr 0", "store ptr nullptr")
            .replace("%x, 16", "%x, 0x10")
            .replace("%a, -1", "%a, 4294967295")
            .replace("const f64 1.5", "const f64 15e-1")
            .replace("const f32 0.1", "const f32 0.100000001490116119384765625")
            .replace("const f80 12.0L", "const f80 1.2e1")
            .replace("%p, 0", "%p, nullptr")
            .replace("global @three : f64 = 3.0", "global @three : f64 = 0x3")
            .replace("global @half : f32 = 0.5", "global @half : f32 = 5e-1f")
            .replace("(%a, 7, 1.5)", "(%a, 0x7, 1.50)")
        )
        self.assertNotEqual(generated, self.LITERALS)
        result = self.compare(self.LITERALS, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_distinguishes_literals_by_value(self):
        for original, changed in (
            ("%x, 16", "%x, 17"),
            ("const f64 1.5", "const f64 1.5000000000000002"),
            ("const f32 0.1", "const f32 0.2"),
            ("store u32 4294967295", "store u32 4294967294"),
            ("(%a, 7, 1.5)", "(%a, 8, 1.5)"),
        ):
            generated = self.LITERALS.replace(original, changed)
            self.assertNotEqual(generated, self.LITERALS)
            result = self.compare(self.LITERALS, generated)
            self.assertNotEqual(result.returncode, 0, changed)
            self.assertIn("does not match reference", result.stdout, changed)

    COMMUTATIVE = """function @main(%left : i64, %right : i64) -> i64 {
  block ^entry:
    %sum = binary add i64 %left, %right
    %product = binary mul i64 %sum, 3
    %mask = binary and i64 %product, %left
    %either = binary or i64 %mask, %right
    %flip = binary xor i64 %either, 1
    %same = cmp eq i64 %flip, %left
    %other = cmp ne i64 %right, 0
    %difference = binary sub i64 %left, %right
    %less = cmp lt i64 %left, %right
    return i64 %difference
}
"""

    def test_relaxed_compare_accepts_commutative_operands_in_either_order(self):
        generated = (
            self.COMMUTATIVE
            .replace("add i64 %left, %right", "add i64 %right, %left")
            .replace("mul i64 %sum, 3", "mul i64 3, %sum")
            .replace("and i64 %product, %left", "and i64 %left, %product")
            .replace("or i64 %mask, %right", "or i64 %right, %mask")
            .replace("xor i64 %either, 1", "xor i64 1, %either")
            .replace("eq i64 %flip, %left", "eq i64 %left, %flip")
            .replace("ne i64 %right, 0", "ne i64 0, %right")
        )
        result = self.compare(self.COMMUTATIVE, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_keeps_non_commutative_operand_order(self):
        for original, changed in (
            ("sub i64 %left, %right", "sub i64 %right, %left"),
            ("lt i64 %left, %right", "lt i64 %right, %left"),
        ):
            generated = self.COMMUTATIVE.replace(original, changed)
            result = self.compare(self.COMMUTATIVE, generated)
            self.assertNotEqual(result.returncode, 0, changed)
            self.assertIn("does not match reference", result.stdout, changed)

    def test_relaxed_compare_accepts_a_second_metadata_group(self):
        reference = REFERENCE.replace(
            ") -> i64 {", ") -> i64 [unwind=no, inline_hint=yes] {", 1
        )
        generated = REFERENCE.replace(
            ") -> i64 {", ") -> i64 [inline_hint=yes] [unwind=no] {", 1
        )
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    # The facts the C++ lowering states (pa8/lowir.md, "Facts The C++
    # Lowering States") are compared on the source-to-LowIR lanes and left
    # uncompared on PA32's optimized lanes, where an optimizer may add them.
    FACTS = """declare function @helper(%p : ptr [pass=by_address, object_bytes=4]) -> i32 [unwind=no]

global @limit : i32 [storage=readonly] = 3

function @twice(%x : i32) -> i32 [unwind=no, inline_hint=yes] {
  block ^entry:
    %y = binary add nowrap i32 %x, %x
    return i32 %y
}

function @main() -> i32 [role=entry] {
  slot $value : i32

  block ^entry:
    store typed i32 1, $value
    %p = addr $value
    %a = call i32 @helper(%p)
    %b = call i32 @twice(%a)
    %fp = addr @twice
    %c = call i32 %fp(%b) as (%x : i32) -> i32 [unwind=no]
    %g = addr @limit
    %l = load typed i32 %g
    %r = binary add nowrap i32 %c, %l
    return i32 %r
}
"""

    UNSOUND_OR_MISSING_FACTS = (
        # main neither returns nothing nor reads no memory
        ("-> i32 [role=entry] {", "-> i32 [role=entry, effects=readnone, return=noreturn] {"),
        ("object_bytes=4", "object_bytes=999"),
        ("-> i32 [unwind=no]\n", "-> i32\n"),
        ("[unwind=no, inline_hint=yes]", "[unwind=no]"),
        ("[storage=readonly] ", ""),
        ("as (%x : i32) -> i32 [unwind=no]", "as (%x : i32) -> i32"),
        ("[pass=by_address, object_bytes=4]", "[pass=by_address, alias=noalias, object_bytes=4]"),
    )

    def test_relaxed_compare_rejects_unsound_and_missing_facts(self):
        for original, changed in self.UNSOUND_OR_MISSING_FACTS:
            self.assertIn(original, self.FACTS)
            generated = self.FACTS.replace(original, changed, 1)
            result = self.compare(self.FACTS, generated)
            self.assertNotEqual(result.returncode, 0, changed)
            self.assertIn("does not match reference", result.stdout, changed)

    def test_optimized_compare_leaves_facts_uncompared(self):
        for original, changed in self.UNSOUND_OR_MISSING_FACTS:
            generated = self.FACTS.replace(original, changed, 1)
            result = self.compare(self.FACTS, generated, mode="lowir_optimized_t")
            self.assertEqual(result.returncode, 0, changed + "\n" + result.stdout + result.stderr)

    def test_relaxed_compare_reads_fact_items_in_any_order_and_grouping(self):
        generated = (
            self.FACTS
            .replace("[unwind=no, inline_hint=yes]", "[inline_hint=yes] [unwind=no]")
            .replace("[pass=by_address, object_bytes=4]", "[object_bytes=4, pass=by_address]")
        )
        result = self.compare(self.FACTS, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    ELISION = """declare function @copy(%dst : ptr [object_bytes=4], %src : ptr [pass=by_address, object_bytes=4]) -> void

function @main(%dst : ptr, %src : ptr) -> i64 {
  block ^entry:
    call void @copy(%dst, %src) [elision=copy]
    return i64 0
}
"""

    def test_relaxed_compare_requires_copy_elision_permission(self):
        generated = self.ELISION.replace(" [elision=copy]", "")
        result = self.compare(self.ELISION, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)
        result = self.compare(self.ELISION, generated, mode="lowir_optimized_t")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_lowir_sanity_reports_a_duplicate_metadata_key(self):
        reference = """function @helper() -> i64 [unwind=no] {
  block ^entry:
    return i64 1
}

function @main() -> i64 {
  block ^entry:
    %value = call i64 @helper()
    return i64 %value
}
"""
        generated = reference.replace("[unwind=no] {", "[unwind=no] [unwind=may] {")
        result = self.compare(reference, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("duplicate function metadata key 'unwind'", result.stdout)
        self.assertNotIn("ARRAY reference", result.stdout + result.stderr)

    def test_seam_rewrites_the_comparison_absorbs_canonicalize_to_the_original(self):
        rewriter = REPO_ROOT / "scripts" / "lowir_seam_rewrite.py"
        sample = self.LITERALS + "\n" + self.COMMUTATIVE.replace("@main", "@second")
        for mode in ("R11-hex-literal", "R16-literal-spelling", "R15-cmp-commute", "R17-binary-commute"):
            with tempfile.TemporaryDirectory() as temporary:
                source = Path(temporary) / "in.lowir"
                target = Path(temporary) / "out.lowir"
                source.write_text(sample, encoding="utf-8")
                subprocess.run(["python3", str(rewriter), mode, str(source), str(target)], check=True)
                rewritten = target.read_text(encoding="utf-8")
            self.assertNotEqual(rewritten, sample, mode)
            result = self.compare(sample, rewritten)
            self.assertEqual(result.returncode, 0, mode + "\n" + result.stdout + result.stderr)

    TWINS = """function @main() -> i64 {
  block ^entry:
    %first = call i64 @left_twin(1)
    %second = call i64 @right_twin(2)
    %sum = binary add i64 %first, %second
    return i64 %sum
}

function @left_twin(%value : i64) -> i64 {
  block ^entry:
    return i64 %value
}

function @right_twin(%value : i64) -> i64 {
  block ^entry:
    return i64 %value
}
"""

    def test_relaxed_compare_pairs_same_shape_helpers_by_first_reference(self):
        # The two helpers have one shape; only their call sites tell them
        # apart.  Emitting them in the other order, under other names, with an
        # unused declaration in front, changes nothing the comparison reads.
        generated = (
            "declare function @unused_helper(%argument : i64) -> void\n\n"
            + self.TWINS.replace("@left_twin", "@helper_b").replace("@right_twin", "@helper_a")
        )
        head, first, second = generated.split("\nfunction @helper_", 2)
        generated = head + "\nfunction @helper_" + second.rstrip("\n") + "\n\nfunction @helper_" + first.rstrip("\n") + "\n"
        result = self.compare(self.TWINS, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_relaxed_compare_keeps_call_sites_of_same_shape_helpers_apart(self):
        # Calling the same helper twice is not calling two helpers once each.
        generated = self.TWINS.replace("call i64 @right_twin(2)", "call i64 @left_twin(2)")
        result = self.compare(self.TWINS, generated)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)

    def test_relaxed_compare_pairs_same_name_functions_whatever_their_parameter_names(self):
        reference = """global @entry_point : ptr = addr @worker

function @main() -> i64 {
  block ^entry:
    return i64 0
}

function @worker(%first : ptr, %count : i64) -> void {
  block ^entry:
    return void
}
"""
        generated = reference.replace("%first : ptr, %count : i64", "%p0 : ptr, %p1 : i64")
        result = self.compare(reference, generated)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        # A second definition of that shape is an entry the reference lacks.
        generated = generated.replace(
            "function @main() -> i64 {",
            "function @helper(%p0 : ptr, %p1 : i64) -> void {\n  block ^entry:\n    return void\n}\n\nfunction @main() -> i64 {",
        )
        result = self.compare(reference, generated)
        self.assertNotEqual(result.returncode, 0)

    def test_debuginfo_compare_relaxes_names_and_keeps_locations(self):
        result = self.compare(DEBUG_REFERENCE, DEBUG_RENAMED, mode="lowir_debuginfo_t")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_debuginfo_compare_requires_every_location(self):
        dropped = DEBUG_RENAMED.replace(" !dbg(main.cpp, 2, 3)", "")
        result = self.compare(DEBUG_REFERENCE, dropped, mode="lowir_debuginfo_t")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("debug locations do not match reference (missing !dbg(main.cpp, 2, 3))",
                      result.stdout)
        undebugged = re.sub(r" !dbg\([^)]*\)", "", DEBUG_RENAMED)
        result = self.compare(DEBUG_REFERENCE, undebugged, mode="lowir_debuginfo_t")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing !dbg(main.cpp, 1, 1) x2", result.stdout)

    def test_debuginfo_compare_rejects_changed_and_extra_locations(self):
        moved = DEBUG_RENAMED.replace("!dbg(main.cpp, 2, 3)", "!dbg(main.cpp, 2, 4)")
        result = self.compare(DEBUG_REFERENCE, moved, mode="lowir_debuginfo_t")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing !dbg(main.cpp, 2, 3); unexpected !dbg(main.cpp, 2, 4)",
                      result.stdout)
        undebugged = re.sub(r" !dbg\([^)]*\)", "", DEBUG_REFERENCE)
        result = self.compare(undebugged, DEBUG_RENAMED, mode="lowir_debuginfo_t")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unexpected !dbg(main.cpp, 1, 1) x2", result.stdout)

    def test_debuginfo_compare_keeps_each_location_on_its_instruction(self):
        shifted = DEBUG_RENAMED.replace(
            "%v1 = load typed i32 $p\n", "%v1 = load typed i32 $p !dbg(main.cpp, 2, 3)\n"
        ).replace("return i32 %v2 !dbg(main.cpp, 2, 3)", "return i32 %v2")
        result = self.compare(DEBUG_REFERENCE, shifted, mode="lowir_debuginfo_t")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match reference", result.stdout)


DEBUG_REFERENCE = """function @add1(%x : i32) -> i32 !dbg(main.cpp, 1, 1) {
  slot $x : i32

  block ^entry:
    store typed i32 %x, $x !dbg(main.cpp, 1, 1)
    %t1 = load typed i32 $x
    %t2 = binary add nowrap i32 %t1, 1
    return i32 %t2 !dbg(main.cpp, 2, 3)
}
"""


DEBUG_RENAMED = """function @add1(%a : i32) -> i32 !dbg(main.cpp, 1, 1) {
  slot $p : i32

  block ^start:
    store typed i32 %a, $p !dbg(main.cpp, 1, 1)
    %v1 = load typed i32 $p
    %v2 = binary add nowrap i32 %v1, 1
    return i32 %v2 !dbg(main.cpp, 2, 3)
}
"""


if __name__ == "__main__":
    unittest.main()


LOWIR2NATIVE = REPO_ROOT / "dev" / "lowir2native"


@unittest.skipUnless(LOWIR2NATIVE.exists(), "needs the built dev/lowir2native")
class OptimizedLowIrBehaviorTest(unittest.TestCase):
    """PA32's optimized lanes run a handwritten fixture with an entry
    function before and after optimization through the reference backend:
    an optimizer that changes the program's answer fails its budgets."""

    FOLD = """function @main() -> i64 [role=entry] {
  block ^entry:
    %a = binary add i64 3, 4
    return i64 %a
}
"""
    FOLDED = """function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 7
}
"""
    STORE = """global @g : i64 = 0

function @main() -> i64 [role=entry] {
  block ^entry:
    store i64 5, @g
    %v = load i64 @g
    return i64 %v
}
"""
    STORED = """global @g : i64 = 0

function @main() -> i64 [role=entry] {
  block ^entry:
    store i64 5, @g
    return i64 5
}
"""
    BRANCH = """function @main() -> i64 [role=entry] {
  block ^entry:
    %c = cmp eq i64 1, 1
    branch %c, ^yes, ^no
  block ^yes:
    return i64 3
  block ^no:
    return i64 4
}
"""
    HELPER = """function @add(%x : i64) -> i64 {
  block ^entry:
    %a = binary add i64 %x, 0
    return i64 %a
}
"""

    def compare(self, source, reference, generated, driver=None):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "case.t").write_text(source, encoding="utf-8")
            if driver is not None:
                (root / "case.driver.lowir").write_text(driver, encoding="utf-8")
            (root / "case.ref").write_text(reference, encoding="utf-8")
            (root / "case.my").write_text(generated, encoding="utf-8")
            (root / "case.ref.expect").write_text("instructions <= 20\n", encoding="utf-8")
            for suffix in ("ref", "my"):
                (root / f"case.{suffix}.exit_status").write_text("EXIT_SUCCESS\n", encoding="utf-8")
            environment = os.environ.copy()
            for name in ("KEEP_GOING", "CPPGM_CHECK_AUTO_KEEP_GOING", "CPPGM_BATCH_TESTS",
                         "WRAPPED_BATCH_STDIN", "CPPGM_TEST_COUNTS_FILE", "CPPGM_LOWER_CHECK_APP",
                         "CPPGM_BEHAVIOR_CHECK_APP", "CPPGM_LOWIR_DIRECT_TEXT_COMPARE"):
                environment.pop(name, None)
            environment["CPPGM_LOWIR_BEHAVIOR_APP"] = str(LOWIR2NATIVE)
            return subprocess.run(
                ["perl", str(COMPARE), "lowir_optimized_t", "ref", "my", str(root / "case.t")],
                cwd=root, env=environment, capture_output=True, text=True)

    def test_a_correct_optimization_passes(self):
        result = self.compare(self.FOLD, self.FOLDED, self.FOLDED)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_a_changed_result_fails(self):
        result = self.compare(self.FOLD, self.FOLDED, self.FOLDED.replace("return i64 7", "return i64 0"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("the optimized program exited with status 0; the input's program exited with status 7",
                      result.stdout)

    def test_a_deleted_observable_store_fails(self):
        result = self.compare(self.STORE, self.STORED, self.STORED)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        result = self.compare(self.STORE, self.STORED,
                              self.STORED.replace("    store i64 5, @g\n    return i64 5", "    return i64 0"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 0; the input's program exited with status 5", result.stdout)

    def test_an_inverted_branch_fails(self):
        result = self.compare(self.BRANCH, self.BRANCH, self.BRANCH)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        result = self.compare(self.BRANCH, self.BRANCH, self.BRANCH.replace("^yes, ^no", "^no, ^yes"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 4; the input's program exited with status 3", result.stdout)

    def test_an_output_the_backend_rejects_fails(self):
        result = self.compare(self.FOLD, self.FOLDED, self.FOLDED.replace("return i64 7", "return i64 %gone"))
        self.assertNotEqual(result.returncode, 0)

    RETURNS_124 = """function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 124
}
"""
    PARAMETERIZED = """function @main(%x : i64, %flag : i1) -> i64 [role=entry] {
  block ^entry:
    return i64 %x
}
"""
    CATCHES = """declare function @may_throw() -> void
function @main() -> i64 {
  block ^entry:
    eh_try ^handler
    call void @may_throw()
    eh_end
    return i64 1

  block ^handler:
    %e = exception i64
    return i64 2
}
"""
    NEEDS_POINTER = """declare function @source() -> ptr
function @main() -> i64 {
  block ^entry:
    %p = call ptr @source()
    return i64 0
}
"""
    DRIVER = """function @main() -> i64 [role=entry] {
  block ^entry:
    %five = const i64 5
    %r = call i64 @add(%five)
    return i64 %r
}
"""

    def test_a_helper_only_fixture_is_driven(self):
        result = self.compare(self.HELPER, self.HELPER, self.HELPER)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("behaviour: ran 1 input (1 through a generated driver)", result.stdout)
        result = self.compare(self.HELPER, self.HELPER, self.HELPER.replace("%x, 0", "%x, 1"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 4; the input's program exited with status 3", result.stdout)

    def test_a_parameterized_entry_is_driven(self):
        result = self.compare(self.PARAMETERIZED, self.PARAMETERIZED, self.PARAMETERIZED)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        result = self.compare(self.PARAMETERIZED, self.PARAMETERIZED,
                              self.PARAMETERIZED.replace("return i64 %x", "return i64 0"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 0; the input's program exited with status 3", result.stdout)

    def test_an_exit_status_of_124_is_not_a_timeout(self):
        result = self.compare(self.RETURNS_124, self.RETURNS_124, self.RETURNS_124.replace("i64 124", "i64 0"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 0; the input's program exited with status 124", result.stdout)

    def test_a_declared_helper_gets_quiet_and_throwing_providers(self):
        result = self.compare(self.CATCHES, self.CATCHES, self.CATCHES)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("1 with generated providers; 1 with a provider that throws", result.stdout)
        result = self.compare(self.CATCHES, self.CATCHES, self.CATCHES.replace("return i64 2", "return i64 3"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 3; the input's program exited with status 2 (with a provider that throws)",
                      result.stdout)

    def test_a_helper_no_provider_can_define_is_reported(self):
        result = self.compare(self.NEEDS_POINTER, self.NEEDS_POINTER, self.NEEDS_POINTER)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("not run: 1 with a declared @source returning ptr, which no generated provider can define",
                      result.stdout)

    def test_a_driver_sidecar_replaces_the_generated_driver(self):
        result = self.compare(self.HELPER, self.HELPER, self.HELPER, driver=self.DRIVER)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("behaviour: ran 1 input (1 with its driver sidecar)", result.stdout)
        result = self.compare(self.HELPER, self.HELPER, self.HELPER.replace("%x, 0", "%x, 1"), driver=self.DRIVER)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("exited with status 6; the input's program exited with status 5", result.stdout)

    def test_an_input_the_backend_rejects_fails_the_lane(self):
        broken = self.RETURNS_124.replace("return i64 124", "return i32 124")
        result = self.compare(broken, self.RETURNS_124, self.RETURNS_124)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("the input does not build through the reference backend", result.stdout)
