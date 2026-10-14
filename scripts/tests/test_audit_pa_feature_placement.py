#!/usr/bin/env python3

import importlib.util
from pathlib import Path
import sys
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "audit_pa_feature_placement.py"


def load_module():
    spec = importlib.util.spec_from_file_location("audit_pa_feature_placement", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


audit = load_module()


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class AuditPAFeaturePlacementTests(unittest.TestCase):
    def test_required_controls_participate_in_placement_and_numbering(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-controls.") as temp_dir:
            root = Path(temp_dir)
            bad = root / "pa12/tests/controls/536-member-call.cpp"
            write(bad, "struct S {}; int S::*member;\n")
            good = root / "pa12/tests/controls/200-direct-call.cpp"
            write(good, "struct S {};\n")
            backend = root / "pa24/tests/controls/900-backend.t"
            write(backend, "function @class() -> void {}\n")
            regression = root / "pa12/tests/regression/controls/536-private.cpp"
            write(regression, "struct S {}; int S::*member;\n")
            helper = root / "pa12/tests/general/helper.cpp"
            write(helper, "struct S {}; int S::*member;\n")
            self.assertEqual(audit.iter_test_files(root, ["pa12", "pa24"]),
                             [good, bad, backend])
            self.assertEqual(audit.suite_test_files(root / "pa12/tests/controls"),
                             [good, bad])
            self.assertEqual(audit.suite_test_files(root / "pa24/tests/controls"),
                             [backend])
            findings = audit.scan_test_hygiene(root, ["pa12", "pa24"])
            self.assertEqual([(f.path, f.kind) for f in findings],
                             [("pa12/tests/controls/536-member-call.cpp", "test-number")])
            features = audit.load_feature_table(REPO_ROOT / audit.DEFAULT_TRACKER)
            row = audit.row_for(bad, root, features)
            self.assertEqual(row["test_role"], "control")
            member = next(p for p in row["placements"] if p["feature"] == "class.member_pointer")
            self.assertEqual((member["status"], member["owner_pa"]), ("violation", "pa22"))
            self.assertEqual(audit.row_for(backend, root, features)["input_contract"], "lowir-backend")
            misplaced_source = root / "pa24/tests/controls/900-member-source.cpp"
            write(misplaced_source, "struct S {}; int S::*member;\n")
            row = audit.row_for(misplaced_source, root, features)
            self.assertEqual(row["input_contract"], "cpp-source")
            self.assertIn("class.member_pointer", row["detected_features"])

    def test_noinline_is_early_function_metadata_without_hiding_host_attributes(self) -> None:
        features = audit.load_feature_table(REPO_ROOT / audit.DEFAULT_TRACKER)
        for spelling in ["noinline", "__noinline__"]:
            hits = audit.detect_features(f"__attribute__(({spelling})) int f();")
            self.assertIn("function.no_inline", hits)
            self.assertNotIn("host.object_attribute", hits)
            self.assertNotIn("support.attribute", hits)
        self.assertEqual(audit.placement_for(features["function.no_inline"], "pa11", 400)[0], "ok")
        self.assertEqual(audit.placement_for(features["function.no_inline"], "pa10", 400)[0], "violation")
        hits = audit.detect_features('__attribute__((noinline)) __attribute__((section("text"))) int f();')
        self.assertIn("function.no_inline", hits)
        self.assertIn("host.object_attribute", hits)

    def test_dynamic_exception_declarations_are_metadata_not_source_eh(self) -> None:
        for source in (
            "struct B { virtual void f() throw(int, double); };",
            "struct D { void f() const & throw(int); ~D() throw(); };",
            "void (*callback)() throw(int);",
            "struct C { void operator()() throw(int); };",
            "struct C { int operator+(int) const throw(int); };",
            "struct C { bool operator==(const C&) const throw(); };",
            "template<class T> struct C { virtual void f() throw(T); };",
            "void f() throw(decltype(sizeof(int)));",
        ):
            with self.subTest(source=source):
                hits = audit.detect_features(source)
                self.assertIn("function.dynamic_exception_spec", hits)
                self.assertNotIn("exception.try_catch", hits)

    def test_throw_expressions_survive_exception_metadata_masking(self) -> None:
        for statement in (
            "throw 1;", "throw(1);", "if (true) throw(1);",
            "while (false) throw(1);", "for (;;) throw(1);",
            "if ((true)) throw(1);", "return (throw(1), 0);",
            "try { throw(1); } catch (...) { throw; }",
        ):
            with self.subTest(statement=statement):
                source = "int f() throw(int) { " + statement + " }"
                self.assertIn("exception.try_catch", audit.detect_features(source))
        self.assertIn("exception.try_catch", audit.detect_features(
            "void f() throw(int);", "declare function @__cxa_throw() -> void"))

    def test_dynamic_exception_metadata_does_not_hide_generated_eh(self) -> None:
        source = "void f() throw(int);"
        reference = "function @f() { block ^entry: eh_try ^cleanup }"
        self.assertTrue(audit.hidden_eh_lowir_evidence(source, reference))

    def test_concrete_template_qualification_does_not_claim_dependency(self) -> None:
        concrete = (
            "template<class T> struct trait; struct Character {}; "
            "typedef trait<Character>::int_type Integer;"
        )
        self.assertNotIn("template.dependent_name", audit.detect_features(concrete))
        shadowed = (
            "template<class Character> struct Holder; struct Character {}; "
            "typedef Holder<Character>::value_type Value;"
        )
        self.assertNotIn("template.dependent_name", audit.detect_features(shadowed))
        unnamed = "template<int> struct Probe { typedef trait<int>::type Value; };"
        self.assertNotIn("template.dependent_name", audit.detect_features(unnamed))
        for source in (
            concrete + " template<class Char> struct Probe { "
            "int padding; typedef typename trait<Char>::int_type Integer; };",
            concrete + " template<class Char> int probe() { "
            "int padding = 0; return trait<Char>::value; }",
            "template<int Count> struct Probe { int padding; "
            "static const int value = trait<Count>::value; };",
        ):
            with self.subTest(source=source):
                self.assertIn("template.dependent_name", audit.detect_features(source))

    def test_scalar_array_copy_does_not_claim_class_transfer_features(self) -> None:
        reference = """global @values [binding=internal, readonly=yes] = {
  i32 1
  i32 2
}
function @main() -> i32 {
  slot $array : obj<8x4>
  block ^entry:
    %destination = addr $array
    %source = addr @values
    copyobj 8x4 %destination, %source
    return i32 0
}
"""
        hits = audit.detect_features("int main() { int values[2] = {1, 2}; }", reference)
        self.assertNotIn("value.copy_move", hits)
        self.assertNotIn("value.by_value_abi", hits)

    def test_class_transfer_signatures_remain_placement_evidence(self) -> None:
        for name in ("_ZN1CC1ERKS_", "_ZN1CC2EOS_", "_ZN1CaSERKS_", "_ZN1CaSEOS_"):
            with self.subTest(name=name):
                hits = audit.detect_features("", f"declare function @transfer() [object={name}]")
                self.assertIn("value.copy_move", hits)
        for signature in ("%value : ptr [pass=by_value]", "return obj<4x4> %value"):
            with self.subTest(signature=signature):
                self.assertIn("value.by_value_abi", audit.detect_features("", signature))

    def test_parser_syntax_does_not_require_later_semantic_behavior(self) -> None:
        for feature_id, owner in (("exception.try_catch", "pa21"),
                                  ("function.noexcept", "pa6"),
                                  ("support.attribute", "pa29")):
            feature = audit.FeatureMeta(feature_id, owner, 100, "", "")
            self.assertEqual(audit.placement_for(feature, "pa5", 100)[0],
                             "semantic-surface")
        exception = audit.FeatureMeta("exception.try_catch", "pa21", 100, "", "")
        self.assertEqual(audit.placement_for(exception, "pa6", 100)[0], "violation")
        self.assertEqual(audit.placement_for(exception, "pa10", 100)[0], "violation")

    def test_non_cpp_inputs_do_not_claim_cpp_feature_ownership(self) -> None:
        features = {
            "expr.array_pointer": audit.FeatureMeta("expr.array_pointer", "pa10", 100, "", ""),
            "exception.try_catch": audit.FeatureMeta("exception.try_catch", "pa21", 100, "", ""),
        }
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            macro = root / "pa4/tests/macros/100-replacement.t"
            lowir = root / "pa8/tests/spec/100-records.t"
            abi = root / "pa9/tests/abi/100-operator.t"
            source = root / "pa10/tests/general/100-body.t"
            write(macro, "#define TRY try { } catch (...) { }\n")
            write(lowir, "function @main() -> i64 {\n block ^entry:\n return i64 0\n}\n")
            write(abi, "function C::operator int\noperator-terminal plus\n")
            write(source, "int main() { try { } catch (...) { } }\n")
            for path in (macro, lowir, abi):
                row = audit.row_for(path, root, features)
                self.assertEqual(row["placements"], [])
                self.assertNotEqual(row["input_contract"], "cpp-source")
            row = audit.row_for(source, root, features)
            self.assertTrue(any(p["status"] == "violation" for p in row["placements"]))

    def test_function_effect_attributes_keep_their_documented_host_owner(self) -> None:
        feature = audit.FeatureMeta("host.function_effect_attribute", "pa28", 200, "", "")
        hits = audit.detect_features("__attribute__((pure)) int f();\n__attribute__((const)) int g(int);\n")
        self.assertIn(feature.feature_id, hits)
        self.assertNotIn("support.attribute", hits)
        self.assertEqual(audit.placement_for(feature, "pa27", 200)[0], "violation")
        self.assertEqual(audit.placement_for(feature, "pa28", 200)[0], "ok")
        mixed = audit.detect_features("__attribute__((pure)) int f();\n__attribute__((nodebug)) int g();\n")
        self.assertIn("support.attribute", mixed)

    def test_debug_preservation_lane_keeps_hygiene_without_source_ownership(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            debug = root / "pa8/tests/debuginfo/source/100-program.t"
            spec = root / "pa8/tests/spec/100-model.t"
            write(debug, "int main() { return 0; }\n")
            write(spec, "function @main() -> i64 {}\n")
            self.assertEqual(audit.iter_test_files(root, ["pa8"]), [spec])
            self.assertIn(debug, audit.iter_local_test_files(root, ["pa8"]))

    def test_type_and_ast_outputs_do_not_require_procedural_lowering(self) -> None:
        hits = audit.detect_features("double value; int main() {}\n")
        for name in ("lowir.procedural", "lowir.procedural.float_conversion"):
            self.assertIn(name, hits)
            feature = audit.FeatureMeta(name, "pa10", 200, "", "")
            for pa in ("pa5", "pa6", "pa7"):
                self.assertEqual(audit.placement_for(feature, pa, 100)[0],
                                 "semantic-surface")
            self.assertEqual(audit.placement_for(feature, "pa10", 100)[0],
                             "cluster-early")

    def test_using_a_global_name_does_not_imply_a_class_nested_type(self) -> None:
        hits = audit.detect_features("typedef int T; namespace N { using ::T; }")
        self.assertNotIn("class.nested_type", hits)
        nested = audit.detect_features("struct C { typedef int T; }; using T = C::T;")
        self.assertIn("class.nested_type", nested)

    def test_pre_lowir_semantic_surface_does_not_claim_later_runtime_owner(self) -> None:
        class_feature = audit.FeatureMeta(
            "class.basic", "pa11", 100, "", ""
        )
        exception_feature = audit.FeatureMeta(
            "exception.try_catch", "pa21", 100, "", ""
        )

        self.assertEqual(
            audit.placement_for(class_feature, "pa6", 200)[0],
            "semantic-surface",
        )
        self.assertEqual(
            audit.placement_for(class_feature, "pa10", 200)[0],
            "violation",
        )
        self.assertEqual(
            audit.placement_for(exception_feature, "pa6", 200)[0],
            "violation",
        )

    def test_pa7_semantic_output_does_not_claim_lowir_body_ownership(self) -> None:
        procedural = audit.FeatureMeta(
            "lowir.procedural", "pa10", 100, "", ""
        )
        condition = audit.FeatureMeta(
            "stmt.condition_declaration", "pa10", 100, "", ""
        )

        self.assertEqual(
            audit.placement_for(procedural, "pa7", 300)[0],
            "semantic-surface",
        )
        self.assertEqual(
            audit.placement_for(condition, "pa7", 300)[0],
            "semantic-surface",
        )
        self.assertEqual(
            audit.placement_for(condition, "pa6", 300)[0],
            "violation",
        )

    def test_pa17_template_review_retains_prerequisites_for_integration_audit(self) -> None:
        features = {
            "template.deduction_full": audit.FeatureMeta(
                "template.deduction_full", "pa18", 100, "", ""
            ),
            "sfinae": audit.FeatureMeta("sfinae", "pa18", 300, "", ""),
            "template.member_template": audit.FeatureMeta(
                "template.member_template", "pa17", 300, "", ""
            ),
            "template.pack": audit.FeatureMeta(
                "template.pack", "pa15", 200, "", ""
            ),
        }

        review = audit.template_review_for(
            list(features), [], [], 300, "pa18", features
        )

        self.assertEqual(
            review["review_template_concepts"],
            ["function-deduction", "sfinae"],
        )
        self.assertEqual(
            review["integration_template_concepts"],
            [
                "function-deduction",
                "member-template",
                "pack-expansion",
                "sfinae",
            ],
        )
        self.assertEqual(review["integration_template_concept_arity"], 4)

    def test_single_pa17_feature_does_not_gain_integration_concepts(self) -> None:
        features = {
            "sfinae": audit.FeatureMeta("sfinae", "pa18", 300, "", ""),
            "template.substitution": audit.FeatureMeta(
                "template.substitution", "pa18", 300, "", ""
            ),
        }

        review = audit.template_review_for(
            list(features), [], [], 300, "pa18", features
        )

        self.assertEqual(review["integration_template_concepts"], ["sfinae"])
        self.assertEqual(review["integration_template_concept_arity"], 1)

    def test_default_argument_detector_ignores_condition_declarations(self) -> None:
        declarations = "int f(int value = 1);\nvoid g(int = 2);\n"
        conditions = textwrap.dedent(
            """\
            int f() {
              if (int value = get(1)) return value;
              for (int index = 0; index != 2; ++index) {}
              return 0;
            }
            """
        )

        self.assertIn("function.default_argument", audit.detect_features(declarations))
        self.assertNotIn("function.default_argument", audit.detect_features(conditions))

    def test_bodyless_placeholder_return_is_hosted_compatibility(self) -> None:
        deleted = "inline constexpr auto blocked(long double) = delete;\n"
        declaration = "auto inspect(int);\n"
        definition = "auto inspect(int value) { return value; }\n"
        trailing = "auto inspect(int value) -> int;\n"
        nested_trailing = (
            "auto sequence_begin(T & value) -> decltype(value.begin());\n"
        )

        self.assertIn(
            "support.bodyless_placeholder_return",
            audit.detect_features(deleted),
        )
        self.assertIn(
            "support.bodyless_placeholder_return",
            audit.detect_features(declaration),
        )
        self.assertNotIn(
            "support.bodyless_placeholder_return",
            audit.detect_features(definition),
        )
        self.assertNotIn(
            "support.bodyless_placeholder_return",
            audit.detect_features(trailing),
        )
        self.assertNotIn(
            "support.bodyless_placeholder_return",
            audit.detect_features(nested_trailing),
        )

    def test_operator_detector_requires_an_operator_function_id(self) -> None:
        identifier = "int operator_arrow_dispatch_(int value);\n"
        overload = "struct value { value &operator+=(int); };\n"

        self.assertNotIn("operator.overload", audit.detect_features(identifier))
        self.assertIn("operator.overload", audit.detect_features(overload))

    def test_bitfield_detector_does_not_match_short_identifier_in_conditional(self) -> None:
        conditional = textwrap.dedent(
            """\
            char short_array[3];
            char first_array[4];
            char *select_array(int index) {
              return index == 0 ? short_array
                                : index == 1 ? first_array : short_array;
            }
            """
        )
        bitfield = "struct flags { unsigned bits : 3; };\n"

        self.assertNotIn("class.layout.bitfield", audit.detect_features(conditional))
        self.assertIn("class.layout.bitfield", audit.detect_features(bitfield))

    def test_vmi_rtti_requires_cast_or_typeid_source_for_pa22_detection(self) -> None:
        ordinary_polymorphic_rtti = textwrap.dedent(
            """\
            struct base {};
            struct derived : base { virtual int value(); };
            """
        )
        vmi_ref = (
            "declare global @__external_rtti_vtable____vmi_class_type_info "
            "[object=_ZTVN10__cxxabiv121__vmi_class_type_infoE]\n"
        )

        self.assertNotIn(
            "rtti.dynamic_cast.multi_vptr",
            audit.detect_features(ordinary_polymorphic_rtti, vmi_ref),
        )
        self.assertIn(
            "rtti.dynamic_cast.multi_vptr",
            audit.detect_features(
                ordinary_polymorphic_rtti
                + "base *convert(derived *p) { return dynamic_cast<base *>(p); }\n",
                vmi_ref,
            ),
        )

    def test_multiple_inheritance_requires_materialized_object_behavior(self) -> None:
        type_lookup_only = textwrap.dedent(
            """\
            struct left { typedef int marker; };
            struct right { typedef long marker; };
            struct derived : left, right {};
            template<class T> struct probe;
            typedef probe<derived> result;
            """
        )
        materialized = type_lookup_only + "derived value;\n"

        self.assertNotIn(
            "class.inheritance.multiple",
            audit.detect_features(type_lookup_only),
        )
        self.assertIn(
            "class.inheritance.multiple",
            audit.detect_features(materialized),
        )

    def test_hygiene_reports_compile_flags_sidecar(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            test = root / "pa31" / "tests" / "link" / "600-hosted-smoke.t"
            write(test, "int main() { return 0; }\n")
            write(test.with_suffix(".compile.flags"), "-O1\n")

            findings = audit.scan_test_hygiene(root, ["pa31"])
            self.assertEqual(
                [(finding.path, finding.kind) for finding in findings],
                [
                    (
                        "pa31/tests/link/600-hosted-smoke.compile.flags",
                        "compile-flags-sidecar",
                    ),
                ],
            )

    def test_hygiene_reports_early_hosted_eh_rtti_headers(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            pa28_typeinfo = root / "pa28" / "tests" / "general" / "200-typeid-header.t"
            write(pa28_typeinfo, "#include <typeinfo>\nint main() { return 0; }\n")

            pa29_exception = root / "pa29" / "tests" / "compile" / "600-exception-header.t"
            write(pa29_exception, "#include <exception>\nint main() { return 0; }\n")

            pa30_typeinfo = root / "pa30" / "tests" / "compile" / "700-typeinfo-header.t"
            write(pa30_typeinfo, "#include <typeinfo>\nint main() { return 0; }\n")

            findings = audit.scan_test_hygiene(root, ["pa28", "pa29", "pa30"])
            self.assertEqual(
                [(finding.path, finding.kind, finding.evidence) for finding in findings],
                [
                    (
                        "pa28/tests/general/200-typeid-header.t",
                        "early-hosted-eh-rtti-header",
                        "#include <typeinfo>",
                    ),
                    (
                        "pa29/tests/compile/600-exception-header.t",
                        "early-hosted-eh-rtti-header",
                        "#include <exception>",
                    ),
                ],
            )

    def test_hygiene_checks_host_interop_numbered_sources(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            pas = ["pa26", "pa27", "pa28", "pa29", "pa30", "pa31"]
            for pa in pas:
                anchor = root / pa / "tests" / "general" / "300-string.t"
                write(anchor, "")
                write(anchor.with_name("300-string.t.1"), "#include <string>\n")

            findings = audit.scan_test_hygiene(root, pas)
            self.assertEqual(
                [(finding.path, finding.kind, finding.evidence) for finding in findings],
                [
                    (f"{pa}/tests/general/300-string.t.1", "early-hosted-stl",
                     "#include <string>")
                    for pa in pas[:4]
                ],
            )

    def test_hygiene_allows_family_owned_angle_header_override(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            driver = root / "pa25" / "tests" / "general" / "300-include-order.t"
            source = driver.with_name("300-include-order.t.1")
            flags = driver.with_suffix(".flags")
            override = (
                root / "pa25" / "tests" / "general" /
                "300-include-order.inc" / "exception"
            )
            write(driver, "user include precedence\n")
            write(source, "#include <exception>\nint main() { return selected(); }\n")
            write(flags, "-I tests/general/300-include-order.inc\n")
            write(override, "int selected() { return 0; }\n")

            findings = audit.scan_test_hygiene(root, ["pa25"])
            self.assertNotIn(
                "early-hosted-eh-rtti-header",
                [finding.kind for finding in findings],
            )

    def test_hygiene_reports_early_exception_ptr_runtime(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            pa27_exception_ptr = (
                root / "pa27" / "tests" / "general" /
                "200-exception-ptr-runtime.t"
            )
            write(
                pa27_exception_ptr,
                textwrap.dedent(
                    """\
                    namespace std {
                    class exception_ptr;
                    exception_ptr make_exception_ptr(int);
                    void rethrow_exception(exception_ptr);
                    }
                    int main() {
                      std::exception_ptr p = std::make_exception_ptr(1);
                      std::rethrow_exception(p);
                    }
                    """
                ),
            )

            pa31_exception_ptr = (
                root / "pa31" / "tests" / "link" /
                "600-exception-ptr-runtime.t"
            )
            write(
                pa31_exception_ptr,
                "int main() { return 0; }\n",
            )

            findings = audit.scan_test_hygiene(root, ["pa27", "pa31"])
            self.assertEqual(
                [(finding.path, finding.kind, finding.evidence) for finding in findings],
                [
                    (
                        "pa27/tests/general/200-exception-ptr-runtime.t",
                        "early-hosted-exception-runtime",
                        "std::exception_ptr",
                    ),
                ],
            )

    def test_hygiene_reports_early_abi_naming_wording(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            early_path = (
                root / "pa8" / "tests" / "general" /
                "100-dependent-mangling-case.t"
            )
            write(early_path, "int main() { return 0; }\n")

            early_content = (
                root / "pa8" / "tests" / "general" /
                "100-dependent-value-case.t"
            )
            write(early_content, "// ABI mangling should not be named here.\n")

            owner_stage = (
                root / "pa9" / "tests" / "abi" /
                "100-dependent-mangling-case.t"
            )
            write(owner_stage, "target function\n")

            findings = audit.scan_test_hygiene(root, ["pa8", "pa9"])
            self.assertEqual(
                [(finding.path, finding.kind, finding.evidence) for finding in findings],
                [
                    (
                        "pa8/tests/general/100-dependent-mangling-case.t",
                        "early-abi-naming-wording",
                        "path:mangl",
                    ),
                    (
                        "pa8/tests/general/100-dependent-value-case.t",
                        "early-abi-naming-wording",
                        "source:mangl",
                    ),
                ],
            )
    def test_range_for_ignores_scope_qualified_ordinary_for_loop(self) -> None:
        ordinary = "for (std::size_t i = 0u; i < size; ++i) {}"
        range_for = "for (std::size_t value : values) {}"

        self.assertNotIn("support.range_for", audit.detect_features(ordinary))
        self.assertIn("support.range_for", audit.detect_features(range_for))

    def test_lambda_ignores_array_allocation_operators(self) -> None:
        allocation_operators = (
            "static void *operator new[](unsigned long); "
            "static void operator delete[](void *p) noexcept { free(p); }"
        )
        lambda_expression = "[](void *p) noexcept { free(p); }"

        self.assertNotIn(
            "support.lambda",
            audit.detect_features(allocation_operators),
        )
        self.assertIn(
            "support.lambda",
            audit.detect_features(lambda_expression),
        )

    def test_nttp_default_reference_expression_is_not_pointer_nttp(self) -> None:
        default_expression = (
            "template<class T, bool Value = noexcept(declval<T&>())> "
            "struct trait;"
        )
        pointer_parameter = "template<class T, T *Value> struct pointer_trait;"

        self.assertNotIn(
            "template.nttp.pointer_member",
            audit.detect_features(default_expression),
        )
        self.assertIn(
            "template.nttp.pointer_member",
            audit.detect_features(pointer_parameter),
        )

    def test_pointer_nttp_detection_stays_inside_template_parameters(self) -> None:
        for source in (
            "template<class T> T* X<T>::n;",
            "template<class T> T& value(T&);",
            "template<class T = int*> struct X;",
            "template<class T> void consume(T*);",
        ):
            with self.subTest(source=source):
                self.assertNotIn("template.nttp.pointer_member", audit.detect_features(source))
        for source in (
            "template<int *P> struct pointer;",
            "template<int &R> struct reference;",
            "template<int X::*M> struct member;",
            "template<int (*F)(int)> struct function;",
        ):
            with self.subTest(source=source):
                self.assertIn("template.nttp.pointer_member", audit.detect_features(source))

    def test_delegating_constructor_ignores_conditional_prvalues(self) -> None:
        conditional = (
            "int result = value() + (choose ? value() : value());"
        )
        delegating = textwrap.dedent(
            """\
            struct value {
              value(int);
              value() : value(0) {}
            };
            """
        )

        self.assertNotIn(
            "value.delegating_ctor",
            audit.detect_features(conditional),
        )
        self.assertIn(
            "value.delegating_ctor",
            audit.detect_features(delegating),
        )

    def test_dynamic_local_static_ignores_class_static_member_array(self) -> None:
        class_static = textwrap.dedent(
            """\
            struct Entry { constexpr Entry(int) {} };
            template<class T> struct Tables {
              static constexpr Entry rows[1] = {Entry(1)};
            };
            """
        )
        function_local = textwrap.dedent(
            """\
            struct Entry { Entry(int) {} };
            int read() {
              static Entry rows[1] = {Entry(1)};
              return 0;
            }
            """
        )

        self.assertNotIn(
            "lowir.procedural.local_static.dynamic_class",
            audit.detect_features(class_static),
        )
        self.assertIn(
            "lowir.procedural.local_static.dynamic_class",
            audit.detect_features(function_local),
        )

    def test_local_static_ref_detector_ignores_guard_class_symbols(self) -> None:
        source = textwrap.dedent(
            """\
            struct guard { guard(); ~guard(); };
            void build() { guard cleanup; }
            """
        )
        ref_text = textwrap.dedent(
            """\
            declare function @guard__guard(%arg0 : ptr) -> void
            declare function @guard___guard(%arg0 : ptr) -> void
            """
        )

        self.assertNotIn(
            "lowir.procedural.local_static",
            audit.detect_features(source, ref_text),
        )
        self.assertIn(
            "lowir.procedural.local_static",
            audit.detect_features(
                "int read();",
                "global @__local_static__read__value__guard : i64 = zero\n",
            ),
        )

    def test_conversion_template_deduction_requires_a_conversion_use(self) -> None:
        declaration_only = textwrap.dedent(
            """\
            struct box {
              template<class T> operator T() const;
            };
            template<class T> box::operator T() const { return T(); }
            int main() { return 0; }
            """
        )
        copy_initialization = textwrap.dedent(
            """\
            struct box {
              template<class T> operator T() const { return T(); }
            };
            int main() {
              box source;
              int result = source;
              return result;
            }
            """
        )

        self.assertNotIn(
            "template.conversion_deduction",
            audit.detect_features(declaration_only),
        )
        self.assertIn(
            "template.conversion_deduction",
            audit.detect_features(copy_initialization),
        )

    def test_class_template_conversion_member_is_not_conversion_template(self) -> None:
        class_template_member = textwrap.dedent(
            """\
            template<class T>
            struct box {
              operator bool() const;
            };
            template<class T>
            box<T>::operator bool() const { return true; }
            int main() {
              box<int> source;
              return static_cast<bool>(source) ? 0 : 1;
            }
            """
        )

        self.assertNotIn(
            "template.conversion_deduction",
            audit.detect_features(class_template_member),
        )

    def test_lowir_eh_review_reports_hidden_source_to_lowir_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            hidden = root / "pa11" / "tests" / "general" / "100-hidden-cleanup.t"
            write(hidden, "struct X { ~X(); }; int main() { X x; return 0; }\n")
            write(hidden.with_suffix(".ref"), textwrap.dedent(
                """\
                function @main() -> i32 {
                  block ^entry:
                    eh_try ^cleanup
                    eh_end
                    return i32 0

                  block ^cleanup:
                    resume
                }
                """
            ))

            runtime_only = root / "pa12" / "tests" / "general" / "100-runtime-only.t"
            write(runtime_only, "struct X { ~X(); }; int main() { X x; return 0; }\n")
            write(runtime_only.with_suffix(".ref"), textwrap.dedent(
                """\
                declare function @__external_runtime___Unwind_Resume() -> void [role=eh_resume]

                function @main() -> i32 {
                  block ^entry:
                    return i32 0
                }
                """
            ))

            explicit = root / "pa11" / "tests" / "general" / "100-explicit-throw.t"
            write(explicit, "int main() { throw 1; }\n")
            write(explicit.with_suffix(".ref"), "function @main() -> i32 {\n  block ^entry:\n    eh_try ^cleanup\n}\n")

            pa8 = root / "pa8" / "tests" / "spec" / "100-lowir-input.t"
            write(pa8, "function @main() -> i32 { block ^entry: eh_try ^cleanup }\n")
            write(pa8.with_suffix(".ref"), "ok\n")

            pa21 = root / "pa21" / "tests" / "general" / "100-owned-hidden-eh.t"
            write(pa21, "struct X { ~X(); }; int main() { X x; return 0; }\n")
            write(pa21.with_suffix(".ref"), "function @main() -> i32 {\n  block ^entry:\n    eh_try ^cleanup\n}\n")

            findings = audit.scan_lowir_eh_review(root, ["pa8", "pa11", "pa12", "pa21"])
            self.assertEqual(
                [(finding.path, finding.kind) for finding in findings],
                [
                    ("pa11/tests/general/100-hidden-cleanup.t", "eh-control"),
                    ("pa12/tests/general/100-runtime-only.t", "eh-runtime-declaration-only"),
                ],
            )

    def test_companion_source_includes_numbered_host_translation_units(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            anchor = root / "pa26" / "tests" / "general" / "200-numbered-source.t"
            write(anchor, "")
            write(anchor.parent / f"{anchor.name}.1", "int main() { throw 1; }\n")
            write(anchor.parent / f"{anchor.name}.2", "int helper() { return 2; }\n")
            write(anchor.with_suffix(".ref.stdout"), "ignored\n")

            source = audit.companion_source_text_for(anchor)
            self.assertIn("throw 1", source)
            self.assertIn("int helper", source)
            self.assertNotIn("ignored", source)

    def test_generated_lowir_review_probes_only_selected_host_tests(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            selected = root / "pa26" / "tests" / "general" / "200-selected.t"
            unselected = root / "pa26" / "tests" / "general" / "200-unselected.t"
            write(selected, "")
            write(selected.parent / f"{selected.name}.1", "// hidden-eh\nint main() { return 0; }\n")
            write(unselected, "")
            write(unselected.parent / f"{unselected.name}.1", "// hidden-eh\nint main() { return 0; }\n")

            compiler = root / "fake-cppgm++"
            write(compiler, textwrap.dedent(
                """\
                #!/usr/bin/env python3
                from pathlib import Path
                import sys

                output = Path(sys.argv[sys.argv.index("-o") + 1])
                output.write_text("function @main() {\\n  eh_try ^cleanup\\n  eh_end\\n}\\n")
                """
            ))
            compiler.chmod(0o755)

            generated = {}
            findings = audit.scan_generated_lowir_eh_review(
                root,
                compiler,
                [selected],
                generated,
            )
            self.assertEqual(len(findings), 1)
            self.assertEqual(findings[0].path, "pa26/tests/general/200-selected.t")
            self.assertEqual(findings[0].kind, "generated-eh-control")
            self.assertIn("200-selected.t.1:eh_try", findings[0].evidence)
            self.assertIn("function @main", generated["pa26/tests/general/200-selected.t"])

    def test_generated_lowir_uses_checked_reference_feature_detection(self) -> None:
        source = textwrap.dedent(
            """\
            struct guard { ~guard(); };
            void run() {
              guard value;
              try { throw 1; }
              catch (int) { if (true) throw 2; }
            }
            """
        )
        hits = audit.generated_lowir_feature_hits(
            source,
            "function @run() {\n  eh_try ^catch\n  eh_cleanup ^outer\n  eh_end\n}\n",
            "pa26/tests/general/200-handler-branch.t",
        )
        self.assertIn("exception.handler_branch_cleanup", hits)
        self.assertTrue(any(
            evidence.startswith("generated:")
            for evidence in hits["exception.handler_branch_cleanup"].evidence
        ))

    def test_pa26_explicit_exception_source_has_host_object_layer(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            anchor = root / "pa26" / "tests" / "general" / "100-host-object.t"
            write(anchor, "")
            write(anchor.parent / f"{anchor.name}.1", "int main() { throw 1; }\n")
            source = audit.companion_source_text_for(anchor)

            self.assertEqual(
                audit.host_eh_object_evidence(anchor, "pa26", source),
                "harness:cppgm++ -c, source:throw",
            )
            self.assertEqual(audit.host_eh_object_evidence(anchor, "pa21", source), "")

    def test_pa27_host_object_attributes_do_not_require_pa29_attribute_support(self) -> None:
        hits = audit.detect_features(
            "__attribute__((weak)) int value;\n"
            "__attribute__((section(\"data\"))) int placed;\n"
            "__attribute__((visibility(\"default\"))) int exported;\n"
            "__attribute__((noinline)) int function();\n",
            test_path="pa27/tests/general/200-object-attributes.t",
        )
        self.assertIn("host.object_attribute", hits)
        self.assertNotIn("support.attribute", hits)

    def test_visibility_attribute_outside_pa27_retains_pa29_owner(self) -> None:
        hits = audit.detect_features(
            "__attribute__((visibility(\"default\"))) int exported;\n",
            test_path="pa17/tests/general/300-attribute.t",
        )
        self.assertNotIn("host.object_attribute", hits)
        self.assertIn("support.attribute", hits)

    def test_pa27_host_object_anchor_is_an_explicit_layer_assertion(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            anchor = root / "pa27" / "tests" / "general" / "200-host-object.t"
            write(anchor, "# The host-object path must preserve member moves.\n")
            write(anchor.parent / f"{anchor.name}.1", "int main() { return 0; }\n")

            self.assertEqual(
                audit.host_object_interop_evidence(anchor, "pa27"),
                "anchor-contract:host-object",
            )
            self.assertEqual(audit.host_object_interop_evidence(anchor, "pa26"), "")

    def test_pa28_abi_tag_inspection_uses_host_abi_attribute_owner(self) -> None:
        hits = audit.detect_features(
            'struct __attribute__((abi_tag("tag"))) Tagged { ~Tagged(); };',
            test_path="pa28/tests/general/200-host-abi-tag-dtor.t",
        )
        self.assertIn("host.abi_name_attribute", hits)
        self.assertNotIn("support.attribute", hits)

    def test_pa9_abi_facts_are_not_classified_as_later_source_features(self) -> None:
        hits = audit.detect_features(
            "typeinfo named:C\nvtable named:C\noperator-terminal plus\n",
            ref_text="_ZTI1C\n_ZTV1C\n",
            test_path="pa9/tests/abi/200-future-vocabulary.t",
        )
        self.assertEqual(hits, {})

    def test_pa28_builtin_transform_mangling_uses_host_abi_owner(self) -> None:
        hits = audit.detect_features(
            "template<class T> using decay_alias = __decay(T);",
            test_path="pa28/tests/general/200-host-builtin-transform-mangling.t",
        )
        self.assertIn("host.abi_builtin_type", hits)
        self.assertNotIn("template.builtin_traits", hits)

    def test_pa29_run_has_hosted_runtime_owner(self) -> None:
        hits = audit.detect_features(
            "int main() { return 0; }",
            test_path="pa29/tests/run/800-hosted-runtime.t",
        )
        self.assertIn("hosted.runtime_compat", hits)

    def test_dynamic_local_static_with_eh_has_guarded_cleanup_feature(self) -> None:
        source = textwrap.dedent(
            """\
            struct value { value(int); };
            int read() {
              static value item(value(1));
              return 0;
            }
            """
        )
        hits = audit.detect_features(
            source,
            "global @__local_static__read__item = zero\n"
            "function @read() {\n  eh_try ^cleanup\n  eh_end\n}\n",
            "pa21/tests/general/200-guarded-static.t",
        )
        self.assertIn("exception.guarded_static_cleanup", hits)

    def test_aggregate_return_with_eh_has_aggregate_cleanup_feature(self) -> None:
        source = textwrap.dedent(
            """\
            struct member { member(int); ~member(); };
            struct aggregate { member value; };
            aggregate make() { return { member(1) }; }
            """
        )
        hits = audit.detect_features(
            source,
            "function @make() {\n  eh_try ^cleanup\n  eh_end\n}\n",
            "pa21/tests/general/200-aggregate-cleanup.t",
        )
        self.assertIn("exception.aggregate_cleanup", hits)

    def test_class_value_argument_with_eh_has_transfer_cleanup_feature(self) -> None:
        source = textwrap.dedent(
            """\
            struct value {
              value();
              value(const value &);
              ~value();
            };
            int consume(value argument) { return 0; }
            int main() { value source; return consume(source); }
            """
        )
        hits = audit.detect_features(
            source,
            "function @consume(%argument : ptr [pass=by_address]) -> i32 {\n"
            "  call void @value___value(%argument)\n}\n"
            "function @main() {\n  eh_try ^cleanup\n  eh_end\n}\n",
            "pa21/tests/general/200-class-value-argument-cleanup.t",
        )
        self.assertIn("exception.class_value_argument_cleanup", hits)

    def test_class_throw_operand_with_eh_has_operand_cleanup_feature(self) -> None:
        source = textwrap.dedent(
            """\
            struct text { ~text(); };
            struct error { error(text); };
            void raise(bool take) {
              if (take) throw error(text());
              throw 1;
            }
            """
        )
        hits = audit.detect_features(
            source,
            "declare function @__cxa_throw() -> void [role=eh_throw]\n"
            "function @raise() {\n  eh_try ^cleanup\n  eh_end\n}\n",
            "pa21/tests/general/200-throw-operand-cleanup.t",
        )
        self.assertIn("exception.throw_operand_cleanup", hits)

    def test_course_has_thirty_five_assignments(self) -> None:
        self.assertEqual(audit.COURSE_PAS[0], "pa1")
        self.assertEqual(audit.COURSE_PAS[-1], "pa35")
        self.assertEqual(len(audit.COURSE_PAS), 35)
        self.assertEqual(audit.LOCAL_TEST_HYGIENE_PAS, audit.COURSE_PAS)

    def test_feature_owners_exclude_assignments_without_language_features(self) -> None:
        def table(*owners: str) -> dict[str, object]:
            return {
                f"feature.{owner}": audit.FeatureMeta(
                    feature_id=f"feature.{owner}",
                    owner_pa=owner,
                    owner_cluster=100,
                    n3485_ref="",
                    required_detections="",
                )
                for owner in owners
            }

        self.assertEqual(audit.feature_owner_errors(table("pa1", "pa29", "pa33")), [])
        errors = audit.feature_owner_errors(table("pa34", "pa35", "pa36"))
        self.assertEqual(len(errors), 3)
        self.assertIn("feature feature.pa34 names owner pa34, which adds no language feature", errors)
        self.assertIn("feature feature.pa35 names owner pa35, which adds no language feature", errors)
        self.assertTrue(any("pa36" in error and "35 assignments" in error for error in errors))

    def test_live_feature_table_names_only_feature_owning_assignments(self) -> None:
        features = audit.load_feature_table(REPO_ROOT / audit.DEFAULT_TRACKER)
        self.assertEqual(audit.feature_owner_errors(features), [])

    def test_pa35_selection_scans_its_tests_without_findings(self) -> None:
        with tempfile.TemporaryDirectory(prefix="cppgm-placement-audit.") as temp_dir:
            root = Path(temp_dir)
            write(root / "pa35" / "tests" / "behavior" / "narrow_ops.cpp", "int main() { return 0; }\n")
            out = root / "report.md"
            status = audit.main([
                "--root", str(root),
                "--tracker", str(REPO_ROOT / audit.DEFAULT_TRACKER),
                "--pa", "pa35",
                "--fail-on-early",
                "--markdown-out", str(out),
            ])
            self.assertEqual(status, 0)
            self.assertIn("- tests scanned: 0", out.read_text())


if __name__ == "__main__":
    unittest.main()
