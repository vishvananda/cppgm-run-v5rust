#!/usr/bin/env python3
"""Detect PA feature usage and report placement risks.

This is a source/ref audit helper. It is intentionally conservative: a match
means "review this test", not "move this test without reading it".

Source and reference-output matches drive placement failures. Filename matches
are retained as path hints for review, but they do not make CI fail.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import shlex
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable


DEFAULT_TRACKER = Path("scripts/architecture/pa-feature-placement.md")
COURSE_PA_COUNT = 35
COURSE_PAS = tuple(f"pa{i}" for i in range(1, COURSE_PA_COUNT + 1))
# Self-hosting (PA34) and optimizing the compiler (PA35) add no language
# feature, so no feature table row may name either as its owner.
NO_FEATURE_PAS = frozenset({"pa34", "pa35"})
DEFAULT_PAS = tuple(f"pa{i}" for i in range(10, 24))
LOCAL_TEST_HYGIENE_PAS = COURSE_PAS
STRICT_TEMPLATE_PAS = ("pa14", "pa15", "pa17", "pa18", "pa19")
SEMANTIC_ONLY_PA_MAX = 7
PRE_LOWIR_SEMANTIC_PA_MIN = 5
LOWIR_SOURCE_PAS = set(range(10, 24))
SOURCE_EH_LOWIR_OWNER_PA = 21
BACKEND_ONLY_PAS = {24}
EARLY_PLACEMENT_STATUSES = {"violation", "cluster-early"}
VALID_TEST_CLUSTERS = frozenset(range(100, 1000, 100))
HOSTED_STL_OWNER_PA = "pa30"
HOSTED_STL_EARLY_PA_MAX = 29
HOSTED_STL_INTERNAL_INCLUDE_PREFIXES = ("__", "bits/", "ext/")
HOST_ABI_EH_COMPONENT_HEADERS = set()
HOSTED_EH_RTTI_HEADERS = {
    "exception",
    "typeinfo",
}
HOSTED_EH_RTTI_HEADER_OWNER_PA = "pa30"
HOSTED_EH_RTTI_HEADER_EARLY_PA_MAX = 29
HOSTED_EXCEPTION_RUNTIME_OWNER_PA = "pa31"
HOSTED_EXCEPTION_RUNTIME_EARLY_PA_MAX = 30
ABI_NAMING_OWNER_PA = "pa9"
ABI_NAMING_EARLY_PA_MAX = 8
ABI_NAMING_WORD_RE = re.compile(r"mangl", re.IGNORECASE)
HOSTED_STL_HEADERS = {
    "algorithm",
    "any",
    "array",
    "atomic",
    "barrier",
    "bit",
    "bitset",
    "charconv",
    "chrono",
    "codecvt",
    "compare",
    "complex",
    "concepts",
    "condition_variable",
    "coroutine",
    "deque",
    "execution",
    "filesystem",
    "format",
    "forward_list",
    "fstream",
    "functional",
    "future",
    "initializer_list",
    "iomanip",
    "ios",
    "iosfwd",
    "iostream",
    "istream",
    "iterator",
    "latch",
    "limits",
    "list",
    "locale",
    "map",
    "memory",
    "memory_resource",
    "mutex",
    "new",
    "numbers",
    "numeric",
    "optional",
    "ostream",
    "queue",
    "random",
    "ranges",
    "ratio",
    "regex",
    "scoped_allocator",
    "semaphore",
    "set",
    "shared_mutex",
    "source_location",
    "span",
    "sstream",
    "stack",
    "stdexcept",
    "stop_token",
    "streambuf",
    "string",
    "string_view",
    "strstream",
    "syncstream",
    "system_error",
    "thread",
    "tuple",
    "type_traits",
    "typeindex",
    "unordered_map",
    "unordered_set",
    "utility",
    "valarray",
    "variant",
    "vector",
    "version",
}
LATE_PLACEMENT_IGNORED_FEATURES = {
    "lowir.procedural",
    "lowir.procedural.float_conversion",
    "call.variadic_promotions",
}
LATE_PLACEMENT_BROAD_FEATURES = {
    "stmt.condition_declaration",
    "expr.reference",
    "expr.array_pointer",
    "expr.cast.builtin",
    "lang.enum",
    "class.basic",
    "template.type",
    "template.class",
    "template.function",
}
LATE_PLACEMENT_ANCHOR_FEATURES = {
    "host.eh_object",
    "host.object_interop",
}
HOST_OBJECT_ATTRIBUTE_NAMES = {"section", "weak"}
PA27_HOST_OBJECT_ATTRIBUTE_NAMES = HOST_OBJECT_ATTRIBUTE_NAMES | {"visibility"}

# PA5-PA7 consume C++ source but stop at AST, type/scope, or call-semantic
# output.  A declaration can therefore contain a class, array, template, or
# value-semantics spelling long before the later source-to-LowIR owner is
# reached.  Those spellings are placement evidence in PA10+, but are only
# semantic-surface inputs in these pre-LowIR assignments. Function declarations
# and floating types also precede lowering; PA7 adds declaration conditions.
# Keep
# exceptions, hosted behavior, ABI, objects, and other LowIR-only families out
# of this exemption so genuinely misplaced tests still fail the audit.
PRE_LOWIR_SEMANTIC_SURFACE_PREFIXES = (
    "call.",
    "class.",
    "constexpr.",
    "expr.",
    "lang.",
    "lookup.",
    "template.",
    "value.",
)
PRE_LOWIR_SEMANTIC_SURFACE_FEATURES = {
    "static_assert",
    "lowir.procedural",
    "lowir.procedural.float_conversion",
}
PA7_SEMANTIC_SURFACE_FEATURES = {
    "stmt.condition_declaration",
}

TEMPLATE_CONCEPT_BY_FEATURE = {
    "template.type": "basic-template",
    "template.class": "basic-template",
    "template.function": "basic-template",
    "template.default_argument": "basic-template",
    "template.dependent_name": "dependent-name",
    "template.friend": "friend-template",
    "template.current_instantiation": "current-instantiation",
    "template.disambiguator": "dependent-name",
    "template.function_partial_ordering": "function-partial-ordering",
    "template.pack": "pack-expansion",
    "template.template_parameter": "template-template-parameter",
    "template.member_template": "member-template",
    "template.nttp": "integral-nttp",
    "template.nttp.pointer_member": "value-nttp",
    "template.explicit_specialization": "explicit-specialization",
    "template.specialization_timing": "specialization-timing",
    "template.partial_specialization": "partial-specialization",
    "template.alias": "alias-template",
    "template.variable": "variable-template",
    "template.current_specialization": "current-specialization",
    "template.explicit_instantiation": "explicit-instantiation",
    "template.specialization_partial_ordering": "specialization-partial-ordering",
    "template.deduction_full": "function-deduction",
    "template.detector_idiom": "detector-idiom",
    "template.substitution": "substitution",
    "template.conversion_deduction": "conversion-deduction",
    "template.constructor_deduction": "constructor-deduction",
    "template.no_eager_instantiation": "no-eager-instantiation",
    "sfinae": "sfinae",
    "template.braced_init_deduction": "braced-init-deduction",
    "template.non_deduced_context": "non-deduced-context",
}

TEMPLATE_LATER_OR_COMPAT_FEATURES = {
    "template.alignas_alignof",
    "template.builtin_traits",
    "template.initializer_list",
    "class.inheritance.multiple",
    "class.member_pointer",
    "support.host_predefined_macro",
    "support.lambda",
    "support.lambda.capture",
    "support.lambda.capture.ref_this",
    "support.preprocessor_probe",
    "support.attribute",
    "support.auto",
    "support.bodyless_placeholder_return",
    "support.range_for",
    "exception.try_catch",
}

TEMPLATE_INTEGRATION_BASIC_SUPPORT = {"basic-template"}
TEMPLATE_PRE_INTEGRATION_SUPPORT = {
    "basic-template",
    "dependent-name",
    "current-instantiation",
}


@dataclass(frozen=True)
class FeatureMeta:
    feature_id: str
    owner_pa: str
    owner_cluster: int
    n3485_ref: str
    required_detections: str


@dataclass(frozen=True)
class FeatureRule:
    feature_id: str
    patterns: tuple[re.Pattern[str], ...]
    all_patterns: bool = False
    use_raw: bool = False
    ref_patterns: tuple[re.Pattern[str], ...] = ()
    path_patterns: tuple[re.Pattern[str], ...] = ()


@dataclass
class FeatureHit:
    feature_id: str
    evidence: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class HygieneFinding:
    path: str
    kind: str
    message: str
    evidence: str = ""


@dataclass(frozen=True)
class LowIREHFinding:
    path: str
    kind: str
    evidence: str


@dataclass(frozen=True)
class TemplateHeader:
    start: int
    end: int
    params: tuple[str, ...]


@dataclass(frozen=True)
class ClassSpan:
    start: int
    body_start: int
    body_end: int
    is_template: bool


def rx(pattern: str) -> re.Pattern[str]:
    return re.compile(pattern, re.MULTILINE | re.DOTALL)


DYNAMIC_CLASS_STATIC_RE = rx(
    r"\bstatic\b[^;{}]*[A-Z][A-Za-z0-9_:<>]*[^;{}]*=\s*\{"
    r"[^{}]*[A-Z][A-Za-z0-9_:<>]*\s*\("
)


RULES: tuple[FeatureRule, ...] = (
    FeatureRule("lowir.procedural", (rx(r"\bint\s+main\s*\("),),
                ref_patterns=(rx(r"^function\s+@",),)),
    FeatureRule("lowir.procedural.local_static",
                (rx(r"\b[A-Za-z_][A-Za-z0-9_:<>*&\s]*\s+[A-Za-z_][A-Za-z0-9_:<>]*\s*\([^;{}]*\)\s*\{[^{}]*\bstatic\b[^{};]*[=;]"),),
                use_raw=True,
                ref_patterns=(rx(r"__local_static__|local_static_(?:init|ready)"),)),
    FeatureRule("lowir.procedural.local_static.dynamic_class",
                (DYNAMIC_CLASS_STATIC_RE,),
                use_raw=True,
                ref_patterns=(rx(r"__local_static__.*(?:C[12]E|copyobj)|(?:C[12]E|copyobj).*__local_static__"),)),
    FeatureRule("lowir.procedural.float_conversion",
                (rx(r"\b(?:float|double|long\s+double)\b|"
                    r"(?<![A-Za-z0-9_])(?:[0-9]+\.[0-9]*|[0-9]*\.[0-9]+|[0-9]+[eE][+-]?[0-9]+)[fFlL]?"),),
                use_raw=True,
                ref_patterns=(rx(r"\b(?:f32|f64|f80)\b|convert\s+(?:fpto|[su]itofp|fpext|fptrunc)"),)),
    FeatureRule("lang.extended_integer", (rx(r"\b__u?int128(?:_t)?\b"),)),
    FeatureRule("stmt.condition_declaration",
                (rx(r"\b(?:if|switch)\s*\(\s*(?:const\s+|volatile\s+|unsigned\s+|signed\s+|long\s+|short\s+|int\b|bool\b|char\b|[A-Za-z_][A-Za-z0-9_:<>]*\s+[&*]?\s*[A-Za-z_])"),)),
    FeatureRule("expr.reference", (rx(r"(?:^|[^\w])(?:const\s+)?[A-Za-z_][A-Za-z0-9_:<>]*\s*&\s*[A-Za-z_]|\bstatic_cast\s*<[^>]*&"),)),
    FeatureRule("expr.array_pointer", (rx(r"\[[^]]*\]|\bnullptr\b|->|\b[A-Za-z_][A-Za-z0-9_]*\s*\+"),)),
    FeatureRule("lang.enum", (rx(r"\benum\b"),)),
    FeatureRule("expr.cast.builtin", (rx(r"\b(?:static_cast|const_cast|reinterpret_cast)\s*<|\([A-Za-z_][A-Za-z0-9_:<>\s*&]*\)\s*[A-Za-z_(0-9]"),)),
    FeatureRule("call.variadic_promotions",
                (rx(r"\b(?:int|void|char|short|long|float|double|signed|unsigned|[A-Za-z_][A-Za-z0-9_:<>*&\s]+)\s+"
                    r"[A-Za-z_][A-Za-z0-9_:]*\s*\((?:\s*\.\.\.|[^)]*,\s*\.\.\.)\s*\)"),)),
    FeatureRule("class.basic",
                (rx(r"(?<!enum )\b(?:class|struct)\s+[A-Za-z_][A-Za-z0-9_]*(?=\s*(?:final\s*)?(?:[:{;]))"),)),
    FeatureRule("class.access_control", (rx(r"\b(?:public|private|protected)\s*:"),)),
    FeatureRule("class.nested_type",
                (rx(r"\b(?:typedef|using(?!\s+::\s*[A-Za-z_]\w*\s*;))\b[^;]*(?:::|class|struct)|\btypename\s+[A-Za-z_][A-Za-z0-9_:<>]*::"),)),
    FeatureRule("class.static_member",
                (rx(r"\b(?:class|struct)\s+\w+(?:\s*:[^{;]+)?\s*\{[^{}]*\bstatic\b[^;{}]*;"),)),
    FeatureRule("class.default_member_initializer",
                (rx(r"\b(?:class|struct)\s+\w+(?:\s*:[^{;]+)?\s*\{[^{}]*\b[A-Za-z_][A-Za-z0-9_:<>*&\s]*\s+[A-Za-z_][A-Za-z0-9_]*\s*=\s*[^;{}]+;"),)),
    FeatureRule("class.aggregate_init",
                (rx(r"(?:=\s*|return\s+)[A-Z][A-Za-z0-9_:<>]*\s*\{[^{};]*\}|"
                    r"\b[A-Z][A-Za-z0-9_:<>]*(?:\s*<[^;{}]*>)?\s+[a-z_][A-Za-z0-9_]*\s*\{[^{};]*\}"),)),
    FeatureRule("class.anonymous_member", (rx(r"\b(?:struct|union)\s*\{"),)),
    FeatureRule("class.friend", (rx(r"\bfriend\b"),)),
    FeatureRule("class.layout.bitfield", (rx(r"\b(?:int|bool|char|unsigned|signed|long|short)\b\s*(?:[A-Za-z_][A-Za-z0-9_]*)?\s*(?<!:):(?!:)\s*[^;{}]+;"),)),
    FeatureRule("class.bitfield.access_or_init",
                (rx(r"\b(?:int|bool|char|unsigned|signed|long|short)\b\s*(?:[A-Za-z_][A-Za-z0-9_]*)?\s*(?<!:):(?!:)\s*[^;{}]+;"),
                 rx(r"\.[A-Za-z_][A-Za-z0-9_]*|:\s*[A-Za-z_][A-Za-z0-9_]*\s*\(|:[^;,(]+[,)]")),
                all_patterns=True),
    FeatureRule("class.inheritance.single", (rx(r"(?<!enum\s)\b(?:class|struct)\s+\w+\s*:\s*(?:public|private|protected)?\s*\w+\s*\{"),)),
    FeatureRule("class.attribute.no_unique_address", (rx(r"\[\[\s*no_unique_address\s*\]\]"),)),
    FeatureRule("expr.new_delete", (rx(r"(?<!operator )\bnew\s+(?!;)|(?<!operator )\bdelete\s+(?:\[\]\s*)?[A-Za-z_(]"),),
                ref_patterns=(rx(r"operator_(?:new|delete)|cppgm_builtin_operator_(?:new|delete)|delete_nonnull"),)),
    FeatureRule("expr.pseudo_destructor", (rx(r"\.\s*~|->\s*~"),)),
    FeatureRule(
        "function.default_argument",
        (
            rx(
                r"\b(?!(?:if|for|while|switch|catch)\b)"
                r"(?:[A-Za-z_~][A-Za-z0-9_:<>~]*|operator\s*[^\s(]+)\s*"
                r"\([^;{}]*(?<!!)(?<![<>=])=(?!=)[^;{}]*\)\s*"
                r"(?:;|\{|const\b|volatile\b|noexcept\b|->|"
                r"=\s*(?:default|delete)\b)"
            ),
        ),
    ),
    FeatureRule("function.noexcept", (rx(r"\bnoexcept\b"),)),
    FeatureRule("function.no_inline", ()),
    FeatureRule("function.dynamic_exception_spec", ()),
    FeatureRule("exception.try_catch",
                (rx(r"\btry\s*\{"), rx(r"\bcatch\s*\("), rx(r"\bthrow\b")),
                ref_patterns=(rx(r"\b__cxa_(?:throw|begin_catch|rethrow)\b|\bexception_selector\b"),)),
    FeatureRule("exception.guarded_static_cleanup", ()),
    FeatureRule("exception.aggregate_cleanup", ()),
    FeatureRule("exception.class_value_argument_cleanup", ()),
    FeatureRule("exception.throw_operand_cleanup", ()),
    FeatureRule("exception.handler_branch_cleanup", ()),
    FeatureRule("host.eh_object", ()),
    FeatureRule("host.object_interop", ()),
    FeatureRule("host.object_attribute", ()),
    FeatureRule("host.function_effect_attribute", ()),
    FeatureRule("host.abi_name_attribute", ()),
    FeatureRule("host.abi_builtin_type", ()),
    FeatureRule("hosted.runtime_compat", ()),
    FeatureRule("lookup.adl", (rx(r"\bfriend\b|\boperator\s+(?!new\b|delete\b)"),)),
    FeatureRule("operator.overload", (rx(r"\boperator(?![A-Za-z0-9_])\s*(?!(?:new|delete)\b)(?:[+\-*/%<>=!&|^~,\[\]()]+|[A-Za-z_][A-Za-z0-9_:<>]*)"),)),
    FeatureRule("class.using_declaration", (rx(r"\busing\s+[A-Za-z_][A-Za-z0-9_:<>]*::[A-Za-z_]"),)),
    FeatureRule("class.inheriting_constructor",
                (rx(r"\busing\s+(?:[A-Za-z_][A-Za-z0-9_:<>]*::)*([A-Za-z_][A-Za-z0-9_]*)::\1\s*;"),)),
    FeatureRule("class.inheritance.multiple", (),
                path_patterns=(rx(r"(?:multiple-inheritance|multiple-base|multibase|diamond|nonprimary-base|downcast-nonprimary|secondary-primary|repeated-base)"),)),
    FeatureRule("class.member_pointer", (rx(r"::\s*\*|\.\*|->\*"),)),
    FeatureRule("class.conversion_operator",
                (rx(r"\boperator\s+(?!new\b|delete\b)(?:const\s+)?[A-Za-z_][A-Za-z0-9_:<>*&\s]*\s*\("),),
                use_raw=True),
    FeatureRule("function.trailing_return", (rx(r"\)\s*->\s*[A-Za-z_][A-Za-z0-9_:<>*&\s]*"),)),
    FeatureRule("support.attribute", (rx(r"\[\[[^]]+\]\]|__attribute__\s*\("),)),
    FeatureRule("lifetime.ctor_dtor",
                (rx(r"\b~[A-Za-z_][A-Za-z0-9_]*\s*\("),),
                path_patterns=(rx(r"(?:lifetime|constructor|destructor|global-constructor|member-object-lifetime)"),)),
    # copyobj also initializes scalar arrays; it is not class-feature evidence.
    FeatureRule("value.copy_move",
                (rx(r"\b(?:copy|move)\b|operator\s*=(?!=)\s*\(\s*(?:const\s+)?[A-Za-z_][A-Za-z0-9_:<>]*\s*(?:&&|&)|"
                    r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*(?:const\s+)?\1\s*(?:&&|&)"),),
                ref_patterns=(rx(r"C[12]E(?:RK|O)S|aSERKS|aSEOS"),)),
    FeatureRule("value.by_value_abi", (rx(r"\b[A-Z][A-Za-z0-9_:<>]*\s+\w+\s*\([^)]*[A-Z][A-Za-z0-9_:<>]*\s+\w+[^)]*\)"),),
                ref_patterns=(rx(r"pass=by_value|return\s+obj<"),)),
    FeatureRule("value.temporary",
                (rx(r"\bconst\s+[A-Za-z_][A-Za-z0-9_:<>]*\s*&\s*\w+\s*=\s*[A-Z][A-Za-z0-9_:<>]*\s*\("),),
                ref_patterns=()),
    FeatureRule("value.ref_qualified_member", (rx(r"\)\s*(?:const\s*)?[&]{1,2}\s*(?:;|\{|->|noexcept)"),)),
    FeatureRule("value.delegating_ctor",
                (rx(r"^[ \t]*(?:[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^;{}()]*>)?\s*::\s*)*"
                    r"([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*:\s*\1\s*"
                    r"\([^;{}]*\)\s*\{"),)),
    FeatureRule("class.union", (rx(r"\bunion\b"),)),
    FeatureRule("expr.array_new_delete", (rx(r"\bnew\s+[^\[]*\[|\bdelete\s*\["),),
                ref_patterns=(rx(r"operator_(?:new|delete)_array|delete_array|array_cookie"),)),
    FeatureRule("lookup.using_directive", (rx(r"\busing\s+namespace\b|\busing\s+[A-Za-z_][A-Za-z0-9_:<>]*::[A-Za-z_]"),)),
    FeatureRule("support.lambda", (rx(r"(?<!operator)(?<!operator new)(?<!operator delete)\[[^]\n]*\]\s*\([^)]*\)\s*(?:mutable\s*)?(?:noexcept\s*)?(?:->|\{)"),)),
    FeatureRule("support.lambda.capture", ()),
    FeatureRule("support.lambda.capture.ref_this", ()),
    FeatureRule("support.range_for",
                (rx(r"\bfor\s*\([^;()]*?(?<!:):(?!:)[^;()]*\)"),)),
    FeatureRule("support.decltype", (rx(r"\bdecltype\s*\("),)),
    FeatureRule("support.auto",
                (rx(r"\bauto\s+(?:[*&]\s*)?[A-Za-z_][A-Za-z0-9_]*\s*(?:=|;|,|\[)|"
                    r"\bauto\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*(?!->)(?:noexcept\s*)?(?:\{|;)"),)),
    FeatureRule(
        "support.bodyless_placeholder_return",
        (
            rx(
                r"\bauto\s+[A-Za-z_][A-Za-z0-9_:]*\s*"
                r"\((?:(?!->)[^;{}])*\)\s*"
                r"(?:(?:const|volatile|noexcept|override|final)\s*)*"
                r"(?:=\s*(?:delete|default)\s*)?;"
            ),
        ),
    ),
    FeatureRule("rtti.typeid", (rx(r"\btypeid\s*\("),)),
    FeatureRule("rtti.dynamic_cast.pointer",
                (rx(r"\bdynamic_cast\s*<"),),
                ref_patterns=(rx(r"__external_runtime____dynamic_cast|object=__dynamic_cast"),)),
    FeatureRule("rtti.dynamic_cast.void",
                (rx(r"\bdynamic_cast\s*<\s*(?:const\s+|volatile\s+)*void\s*\*"),),
                ref_patterns=(rx(r"__external_rtti__void|object=_ZTIv"),)),
    FeatureRule("rtti.dynamic_cast.multi_vptr",
                (),
                ref_patterns=(rx(r"__vmi_class_type_info|_ZTVN10__cxxabiv121__vmi_class_type_infoE"),),
                path_patterns=(rx(r"(?:sibling-dynamic-cast|typeid-nonprimary|nonprimary|virtual-base|vbase)"),)),
    FeatureRule("polymorphic.basic", (rx(r"\bvirtual\b"),),
                ref_patterns=(rx(r"__vtable|_ZTV|__rtti|_ZTI|typeinfo"),)),
    FeatureRule("polymorphic.override_final", (rx(r"\b(?:override|final)\b"),)),
    FeatureRule("polymorphic.vdtor", (rx(r"\bvirtual\s+~"),)),
    FeatureRule("polymorphic.vtable_order", (),
                path_patterns=(rx(r"(?:vtable-order|destructor-slot|slot-merge|vptr-overwrite|vtable-offset)"),)),
    FeatureRule("polymorphic.pointer_adjust",
                (rx(r"\bvirtual\b"), rx(r"\b(?:class|struct)\s+\w+\s*:[^{,]+,[^{]+|static_cast\s*<[^>]*[&*]")),
                all_patterns=True,
                ref_patterns=(rx(r"this_adjust|adjustor|thunk|vbase|vcall"),)),
    FeatureRule("template.type", (rx(r"\btemplate\s*<[^>]*(?:class|typename)\b"),)),
    FeatureRule("template.class", (rx(r"\btemplate\s*<[^>]*>\s*(?:class|struct)\b"),)),
    FeatureRule("template.function",
                (rx(r"\btemplate\s*<[^>]*>[^;{}()]*\b[A-Za-z_][A-Za-z0-9_:<>*&\s]*\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*(?:\{|;)"),)),
    FeatureRule("template.default_argument",
                (rx(r"\btemplate\s*<[^>]*=[^>]*>"),),
                path_patterns=(rx(r"(?:default-template-arg|template-default|progressive-template-defaults|parameter-default)"),)),
    FeatureRule("template.dependent_name",
                (rx(r"\btypename\s+[A-Za-z_][A-Za-z0-9_:<>]*::|[A-Za-z_][A-Za-z0-9_]*<[^>]*>::"),),
                path_patterns=(rx(r"(?:^|[-_/])(?:dependent|typename|template-body)(?:[-_/]|$)"),)),
    FeatureRule("template.friend", (rx(r"\bfriend\b[^;{]*\btemplate\b|\btemplate\s*<[^>]*>[^;{]*\bfriend\b"),)),
    FeatureRule("template.current_instantiation", (),
                path_patterns=(rx(r"current-instantiation|current-owner|source-owner|owner-type|owner-param|owner-key"),)),
    FeatureRule("template.disambiguator",
                (rx(r"\btypename\s+[A-Za-z_][A-Za-z0-9_:<>]*::|(?:\.|->|::)\s*template\s+[A-Za-z_]"),),
                path_patterns=(rx(r"(?:^|[-_/])(?:typename|template-keyword|missing-(?:typename|keyword))(?:[-_/]|$)"),)),
    FeatureRule("template.function_partial_ordering", (),
                path_patterns=(rx(r"(?:function-partial-order|partial-order|partial_order|pack-fallback|more-specialized)"),)),
    FeatureRule("template.alignas_alignof", (rx(r"\balignas\s*\(|\balignof\s*\(|__alignof__\s*\("),)),
    FeatureRule("support.host_predefined_macro",
                (rx(r"\b(?:__GLIBCXX_(?:TYPE|BITSIZE)_INT_N_[A-Za-z0-9_]*|"
                    r"__[A-Za-z0-9_]+_(?:TYPE|MAX|WIDTH)__|"
                    r"__(?:FLT|DBL|LDBL|SIZEOF|ATOMIC|CLANG_ATOMIC|GCC_ATOMIC)_[A-Za-z0-9_]+|"
                    r"__(?:APPLE|APPLE_CC|APPLE_CPP|MACH|linux|unix|ELF|llvm|clang|GNUC|GNUG|VERSION|"
                    r"GXX_EXPERIMENTAL_CXX0X|GXX_WEAK|USER_LABEL_PREFIX|EXCEPTIONS|GXX_RTTI|"
                    r"cpp_exceptions|cpp_ref_qualifiers|cpp_rvalue_references|cpp_static_assert|cpp_rtti|"
                    r"apple_build_version|CHAR_BIT)__|_GNU_SOURCE|_WIN32)\b"),)),
    FeatureRule("support.preprocessor_probe",
                (rx(r"\b__(?:has_feature|has_extension|has_attribute|has_cpp_attribute|has_builtin|"
                    r"is_identifier|building_module|has_include|has_include_next)\b"),)),
    FeatureRule("template.pack", (rx(r"\btemplate\s*<[^>]*\.\.\."),)),
    FeatureRule("template.template_parameter", (rx(r"\btemplate\s*<[^>]*template\s*<"),)),
    FeatureRule("template.member_template",
                (rx(r"\btemplate\s*<[^>]*>\s*template\s*<"),),
                path_patterns=(rx(r"(?:member-template|templated-member)"),)),
    FeatureRule("template.builtin_traits",
                (rx(r"\b__(?:is|has|is_trivially|is_nothrow)_[A-Za-z0-9_]*\b|"
                    r"\b__builtin_(?:is|has|types_compatible_p|classify_type)[A-Za-z0-9_]*|"
                    r"\b__(?:add_(?:lvalue|rvalue)_reference|decay|integer_pack|"
                    r"make_integer_seq|remove_(?:const|cv|cvref|reference|volatile)(?:_t)?|"
                    r"type_pack_element)\b"),)),
    FeatureRule("template.nttp", (rx(r"\btemplate\s*<[^>]*(?:int|bool|char|long|short|unsigned|signed|std::size_t|size_t)\s+\w+"),)),
    # Parsed template headers distinguish NTTP declarators from pointer types
    # following the closing '>' and from type-parameter default arguments.
    FeatureRule("template.nttp.pointer_member", ()),
    FeatureRule("template.explicit_specialization", (rx(r"\btemplate\s*<\s*>\s*"),)),
    FeatureRule("template.specialization_timing", (),
                path_patterns=(rx(r"(?:^|[-_/])(?:stale|refresh|late[-_]|after-instantiation|keeps-definition|"
                                  r"(?:explicit-specialization|specialization)[-_][A-Za-z0-9_-]*redeclaration|"
                                  r"redeclaration[-_][A-Za-z0-9_-]*(?:explicit-specialization|specialization))"),)),
    FeatureRule("constexpr.integral_subset",
                (rx(r"\bconstexpr\b|\bstatic_assert\b|\btemplate\s*<[^>]*(?:int|bool|char|long|short|unsigned|signed)\s+\w+"),)),
    FeatureRule("static_assert", (rx(r"\bstatic_assert\s*\("),)),
    FeatureRule("constexpr.full",
                (rx(r"\bconstexpr\s+[A-Za-z_][A-Za-z0-9_:<>*&\s]+\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{"),)),
    FeatureRule("constexpr.control_flow", (rx(r"\bconstexpr\b"), rx(r"\b(?:if|for|while|do|switch)\b"),), all_patterns=True),
    FeatureRule("constexpr.default_argument",
                (rx(r"\bconstexpr\s+[A-Za-z_][A-Za-z0-9_:<>*&\s]+\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*=[^)]*\)"),)),
    FeatureRule("constexpr.floating", (rx(r"\bconstexpr\b"), rx(r"\b(?:float|double|long\s+double)\b|[0-9]*\.[0-9]+")), all_patterns=True),
    FeatureRule("constexpr.noexcept", (),
                path_patterns=(rx(r"constexpr-noexcept"),)),
    FeatureRule("constexpr.object", (),
                path_patterns=(rx(r"constexpr-object|constexpr-class|constexpr-struct"),)),
    FeatureRule("template.partial_specialization",
                (rx(r"\btemplate\s*<[^>]+>\s*(?:class|struct)\s+[A-Za-z_][A-Za-z0-9_]*\s*<[^>]*>\s*(?!::)"),)),
    FeatureRule("template.alias", (rx(r"\btemplate\s*<[^>]*>\s*using\b"),)),
    FeatureRule("template.variable",
                (rx(r"\btemplate\s*<[^>]*>\s*(?:constexpr\s+|const\s+|static\s+)?"
                    r"(?!using\b)(?:bool|int|char|long|unsigned|signed|[A-Za-z_][A-Za-z0-9_:<>]*)\s+"
                    r"(?!operator\b)\w+\s*="),)),
    FeatureRule("template.current_specialization", (),
                path_patterns=(rx(r"current-specialization"),)),
    FeatureRule("template.explicit_instantiation",
                (rx(r"\bextern\s+template\b"),),
                path_patterns=(rx(r"(?:explicit-instantiation|extern-template)"),)),
    FeatureRule("template.specialization_partial_ordering", (),
                path_patterns=(rx(r"(?:partial-specialization-order|class-partial-order|partial_order_class|function-type-partial-specialization-preference|repeated-argument)"),)),
    FeatureRule("template.deduction_full",
                (rx(r"\btemplate\s*<[^>]*>[^;{}()]*\b[A-Za-z_][A-Za-z0-9_:<>*&\s]*\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*(?:&&|decltype|enable_if|typename\s+[A-Za-z_][A-Za-z0-9_:<>]*::)[^)]*\)"),),
                path_patterns=(rx(r"(?:deduc|forwarding-reference|explicit-template-(?:args|argument|id)|nondeduced|non-deduced|reference-cv|array-bound)"),)),
    FeatureRule("template.detector_idiom",
                (rx(r"\b(?:void_t|detected_or|detector|is_detected)\b"),),
                path_patterns=(rx(r"(?:detected|detector|void-t|void_t)"),)),
    FeatureRule("template.substitution",
                (rx(r"\b(?:enable_if|void_t|sfinae|SFINAE|detected_or|detector|is_detected)\b"),),
                path_patterns=(rx(r"(?:candidate-drop|substitution)"),)),
    FeatureRule("template.conversion_deduction",
                (rx(r"\btemplate\s*<\s*(?:class|typename)\s+"
                    r"([A-Za-z_][A-Za-z0-9_]*)[^>]*>[^;{]*"
                    r"operator\s+\1\b"),
                 rx(r"\bstatic_cast\s*<[^>]+>\s*\(|"
                    r"\b[A-Za-z_][A-Za-z0-9_:<>]*(?:\s*[*&])?\s+"
                    r"[A-Za-z_][A-Za-z0-9_]*\s*=\s*[A-Za-z_][A-Za-z0-9_]*\s*;")),
                all_patterns=True,
                use_raw=True),
    FeatureRule("template.constructor_deduction", (),
                path_patterns=(rx(r"constructor-template|converting-ctor|ctor-template"),)),
    FeatureRule("template.initializer_list", (rx(r"\binitializer_list\b"),)),
    FeatureRule("template.no_eager_instantiation",
                (rx(r"\b(?:no_eager|unevaluated|static_assert\s*\(\s*false|sizeof\s*\([^)]*typename)"),),
                path_patterns=(rx(r"(?:no[-_]?eager|no[-_]?body|does-not-eagerly|not-instantiat|body-skip|unused-body)"),)),
    FeatureRule("sfinae", (rx(r"\b(?:enable_if|void_t|sfinae|SFINAE|detected_or|detector|is_detected)\b"),),
                path_patterns=(rx(r"sfinae|candidate-drop"),)),
    FeatureRule("template.braced_init_deduction",
                (rx(r"\btemplate\s*<[^>]*>[^;{}()]*\b[A-Za-z_][A-Za-z0-9_:<>*&\s]*\s+"
                    r"[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*(?:\{|;).*?"
                    r"\b[A-Za-z_][A-Za-z0-9_]*\s*\(\s*\{"),)),
    FeatureRule("template.non_deduced_context",
                (rx(r"\b(?:non_deduced|nondeduced)\b"),),
                path_patterns=(rx(r"non[-]?deduced|nondeduced"),)),
)


def pa_number(pa: str) -> int | None:
    match = re.fullmatch(r"pa(\d+)", pa)
    return int(match.group(1)) if match else None


def split_markdown_row(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def strip_backticks(value: str) -> str:
    value = value.strip()
    if value.startswith("`") and value.endswith("`"):
        return value[1:-1]
    return value


def load_feature_table(path: Path) -> dict[str, FeatureMeta]:
    features: dict[str, FeatureMeta] = {}
    in_table = False
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("| Feature Family |"):
                in_table = True
                continue
            if not in_table:
                continue
            if line.startswith("| ---"):
                continue
            if not line.startswith("|"):
                if features:
                    break
                continue
            cells = split_markdown_row(line)
            if len(cells) < 6:
                continue
            feature_id = strip_backticks(cells[0])
            owner_pa = strip_backticks(cells[1])
            try:
                owner_cluster = int(cells[2])
            except ValueError:
                continue
            features[feature_id] = FeatureMeta(
                feature_id=feature_id,
                owner_pa=owner_pa,
                owner_cluster=owner_cluster,
                n3485_ref=cells[3],
                required_detections=cells[5],
            )
    if not features:
        raise ValueError(f"no feature table found in {path}")
    return features


def feature_owner_errors(features: dict[str, FeatureMeta]) -> list[str]:
    errors: list[str] = []
    for feature_id, meta in sorted(features.items()):
        if meta.owner_pa not in COURSE_PAS:
            errors.append(
                f"feature {feature_id} names owner {meta.owner_pa}, "
                f"which is not one of the course's {COURSE_PA_COUNT} assignments"
            )
        elif meta.owner_pa in NO_FEATURE_PAS:
            errors.append(
                f"feature {feature_id} names owner {meta.owner_pa}, "
                "which adds no language feature"
            )
    return errors


def strip_comments(source: str) -> str:
    out: list[str] = []
    i = 0
    in_block = False
    in_line = False
    in_string: str | None = None
    while i < len(source):
        c = source[i]
        n = source[i + 1] if i + 1 < len(source) else ""
        if in_line:
            if c == "\n":
                in_line = False
                out.append(c)
            else:
                out.append(" ")
            i += 1
            continue
        if in_block:
            if c == "*" and n == "/":
                out.extend("  ")
                i += 2
                in_block = False
            else:
                out.append("\n" if c == "\n" else " ")
                i += 1
            continue
        if in_string:
            out.append(c)
            if c == "\\" and i + 1 < len(source):
                out.append(source[i + 1])
                i += 2
                continue
            if c == in_string:
                in_string = None
            i += 1
            continue
        if c == "/" and n == "/":
            out.extend("  ")
            in_line = True
            i += 2
            continue
        if c == "/" and n == "*":
            out.extend("  ")
            in_block = True
            i += 2
            continue
        if c in ("'", '"'):
            in_string = c
        out.append(c)
        i += 1
    return "".join(out)


def strip_string_literals(source: str) -> str:
    out: list[str] = []
    i = 0
    in_string: str | None = None
    while i < len(source):
        c = source[i]
        if in_string:
            if c == "\\" and i + 1 < len(source):
                out.extend("  ")
                i += 2
                continue
            out.append(" ")
            if c == in_string:
                in_string = None
            i += 1
            continue
        if c in ("'", '"'):
            in_string = c
            out.append(" ")
            i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def strip_preprocessor_directives(source: str) -> str:
    return re.sub(r"^\s*#.*$", "", source, flags=re.MULTILINE)


def strip_dynamic_exception_specifications(code: str) -> str:
    """Leave throw expressions visible while masking function suffix metadata."""
    if not re.search(r"\bthrow\s*\(", code):
        return code
    stack: list[int] = []
    openings: dict[int, int] = {}
    for index, char in enumerate(code):
        if char == "(":
            stack.append(index)
        elif char == ")" and stack:
            openings[index] = stack.pop()
    out = list(code)
    controls = {"if", "while", "for", "switch", "catch", "sizeof", "alignof",
                "typeid", "decltype", "noexcept", "static_assert"}
    for match in re.finditer(r"\bthrow\s*\(", code):
        prefix = re.sub(r"(?:\s+\b(?:const|volatile)\b|\s*[&]{1,2})+\s*$",
                        "", code[:match.start()]).rstrip()
        if not prefix.endswith(")"):
            continue
        opening = openings.get(len(prefix) - 1)
        if opening is None:
            continue
        name = re.search(r"([A-Za-z_][A-Za-z0-9_]*|>|\]|\))\s*$", code[:opening])
        operator = re.search(r"\boperator\s*[+*/%=!<>&|^~,\-]+\s*$", code[:opening])
        if (not name and not operator) or (name and name.group(1) in controls):
            continue
        start = match.end() - 1
        end = matching_delimiter(code, start, "(", ")")
        if end is not None:
            out[match.start():end + 1] = " " * (end + 1 - match.start())
    return "".join(out)


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="latin-1")


def match_rule_patterns(patterns: tuple[re.Pattern[str], ...],
                        haystack: str,
                        all_patterns: bool,
                        prefix: str) -> list[str]:
    if not patterns:
        return []
    matched: list[str] = []
    for pattern in patterns:
        match = pattern.search(haystack)
        if match:
            evidence = " ".join(match.group(0).split())
            matched.append(f"{prefix}:{evidence[:120]}")
    if all_patterns and len(matched) != len(patterns):
        return []
    return matched


def match_builtin_trait_patterns(patterns: tuple[re.Pattern[str], ...],
                                 haystack: str,
                                 declared_intrinsics: set[str]) -> list[str]:
    matched: list[str] = []
    for pattern in patterns:
        for match in pattern.finditer(haystack):
            evidence = " ".join(match.group(0).split())
            name = re.match(r"(__[A-Za-z_][A-Za-z0-9_]*)", evidence)
            if name and name.group(1) in declared_intrinsics:
                continue
            item = f"source:{evidence[:120]}"
            if item not in matched:
                matched.append(item)
    return matched


def multiple_base_class_names(code: str) -> list[str]:
    class_header = re.compile(
        r"\b(?:class|struct)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+final)?\s*:\s*([^{;]+)\{",
        re.MULTILINE | re.DOTALL,
    )
    names: list[str] = []
    for match in class_header.finditer(code):
        depth_angle = 0
        depth_paren = 0
        depth_bracket = 0
        base_clause = match.group(2)
        index = 0
        while index < len(base_clause):
            char = base_clause[index]
            if char == "<":
                depth_angle += 1
            elif char == ">" and depth_angle and index + 1 < len(base_clause) and base_clause[index + 1] == "=":
                pass
            elif char == ">" and depth_angle:
                depth_angle -= 1
            elif char == "(":
                depth_paren += 1
            elif char == ")" and depth_paren:
                depth_paren -= 1
            elif char == "[":
                depth_bracket += 1
            elif char == "]" and depth_bracket:
                depth_bracket -= 1
            elif char == "," and depth_angle == 0 and depth_paren == 0 and depth_bracket == 0:
                names.append(match.group(1))
                break
            index += 1
    return names


def has_materialized_multiple_inheritance(code: str) -> bool:
    for name in multiple_base_class_names(code):
        escaped = re.escape(name)
        materialized_patterns = (
            rf"\b(?:new|sizeof|alignof)\s*(?:\(\s*)?{escaped}\b",
            rf"\b(?:static_cast|dynamic_cast|reinterpret_cast|const_cast)\s*<[^>]*\b{escaped}\b",
            rf"\b(?:const\s+|volatile\s+)*{escaped}\s*(?:[*&]\s*)?[A-Za-z_][A-Za-z0-9_]*\s*(?:[=;,{{(\[]|$)",
        )
        if any(re.search(pattern, code, re.MULTILINE) for pattern in materialized_patterns):
            return True
    return False


def declared_intrinsic_like_names(code: str) -> set[str]:
    names: set[str] = set()
    for match in re.finditer(
        r"\b(?:class|struct|union)\s+(__[A-Za-z_][A-Za-z0-9_]*)\b",
        code,
        re.MULTILINE | re.DOTALL,
    ):
        names.add(match.group(1))
    for match in re.finditer(
        r"\b(?:template\s*<[^>]*>\s*)?using\s+(__[A-Za-z_][A-Za-z0-9_]*)\b",
        code,
        re.MULTILINE | re.DOTALL,
    ):
        names.add(match.group(1))
    for match in re.finditer(
        r"\btypedef\b[^;]*\s+(__[A-Za-z_][A-Za-z0-9_]*)\s*;",
        code,
        re.MULTILINE | re.DOTALL,
    ):
        names.add(match.group(1))
    for match in re.finditer(
        r"\b(?:static\s+|constexpr\s+|inline\s+|extern\s+|friend\s+|virtual\s+)*"
        r"(?:bool|int|char|short|long|unsigned|signed|void|[A-Za-z_][A-Za-z0-9_:<>]*)\s+"
        r"(__[A-Za-z_][A-Za-z0-9_]*)\s*\(",
        code,
        re.MULTILINE | re.DOTALL,
    ):
        names.add(match.group(1))
    for match in re.finditer(
        r"\b(?:template\s*<[^>]*>\s*)?(?:static\s+)?(?:constexpr\s+|const\s+)?"
        r"(?:bool|int|char|short|long|unsigned|signed|[A-Za-z_][A-Za-z0-9_:<>]*)\s+"
        r"(__[A-Za-z_][A-Za-z0-9_]*)\s*(?:=|;)",
        code,
        re.MULTILINE | re.DOTALL,
    ):
        names.add(match.group(1))
    return names


def overloaded_arrow_star_without_member_pointer(code: str) -> bool:
    if not re.search(r"\boperator\s*->\s*\*", code):
        return False
    return not re.search(r"::\s*\*|\.\*", code)


def matching_delimiter(code: str, start: int, open_char: str, close_char: str) -> int | None:
    depth = 0
    i = start
    while i < len(code):
        c = code[i]
        if c == open_char:
            depth += 1
        elif c == close_char:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return None


def split_top_level_commas(text: str) -> tuple[str, ...]:
    parts: list[str] = []
    start = 0
    angle = paren = bracket = brace = 0
    for i, c in enumerate(text):
        if c == "<":
            angle += 1
        elif c == ">" and angle:
            angle -= 1
        elif c == "(":
            paren += 1
        elif c == ")" and paren:
            paren -= 1
        elif c == "[":
            bracket += 1
        elif c == "]" and bracket:
            bracket -= 1
        elif c == "{":
            brace += 1
        elif c == "}" and brace:
            brace -= 1
        elif c == "," and not (angle or paren or bracket or brace):
            parts.append(text[start:i].strip())
            start = i + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return tuple(parts)


def before_top_level_equal(text: str) -> str:
    angle = paren = bracket = brace = 0
    for i, c in enumerate(text):
        if c == "<":
            angle += 1
        elif c == ">" and angle:
            angle -= 1
        elif c == "(":
            paren += 1
        elif c == ")" and paren:
            paren -= 1
        elif c == "[":
            bracket += 1
        elif c == "]" and bracket:
            bracket -= 1
        elif c == "{":
            brace += 1
        elif c == "}" and brace:
            brace -= 1
        elif c == "=" and not (angle or paren or bracket or brace):
            return text[:i].rstrip()
    return text


def has_top_level_equal(text: str) -> bool:
    return len(before_top_level_equal(text)) != len(text)


def find_template_headers(code: str) -> list[TemplateHeader]:
    headers: list[TemplateHeader] = []
    for match in re.finditer(r"\btemplate\b", code):
        i = match.end()
        while i < len(code) and code[i].isspace():
            i += 1
        if i >= len(code) or code[i] != "<":
            continue
        end = matching_delimiter(code, i, "<", ">")
        if end is None:
            continue
        params = split_top_level_commas(code[i + 1:end])
        headers.append(TemplateHeader(match.start(), end + 1, params))
    return headers


def template_header_before(headers: Iterable[TemplateHeader], pos: int, code: str) -> TemplateHeader | None:
    candidate: TemplateHeader | None = None
    for header in headers:
        if header.end <= pos and code[header.end:pos].strip() == "":
            if candidate is None or header.start > candidate.start:
                candidate = header
    return candidate


def match_dependent_name_patterns(patterns: tuple[re.Pattern[str], ...],
                                  code: str) -> list[str]:
    scopes: list[tuple[int, int, set[str]]] = []
    for header in find_template_headers(code):
        names: set[str] = set()
        for parameter in header.params:
            declaration = before_top_level_equal(parameter)
            name = re.search(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*$", declaration)
            if name and name.group(1) not in {
                "class", "typename", "bool", "char", "wchar_t", "char16_t",
                "char32_t", "short", "int", "long", "signed", "unsigned",
                "float", "double", "void", "auto",
            }:
                names.add(name.group(1))
        semi = code.find(";", header.end)
        brace = code.find("{", header.end)
        if brace != -1 and (semi == -1 or brace < semi):
            end = matching_delimiter(code, brace, "{", "}")
        else:
            end = semi if semi != -1 else len(code)
        scopes.append((header.start, end if end is not None else len(code), names))
    matched: list[str] = []
    for pattern in patterns:
        for match in pattern.finditer(code):
            spelling = match.group(0)
            identifiers = set(re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", spelling))
            if not spelling.startswith("typename") and not any(
                start <= match.start() < end and identifiers & names
                for start, end, names in scopes
            ):
                continue
            evidence = "source:" + " ".join(spelling.split())
            if evidence not in matched:
                matched.append(evidence)
    return matched


def find_class_spans(code: str, headers: Iterable[TemplateHeader]) -> list[ClassSpan]:
    spans: list[ClassSpan] = []
    for match in re.finditer(r"(?<!enum\s)\b(?:class|struct)\s+[A-Za-z_][A-Za-z0-9_]*", code):
        open_idx = code.find("{", match.end())
        semi_idx = code.find(";", match.end())
        if open_idx == -1 or (semi_idx != -1 and semi_idx < open_idx):
            continue
        close_idx = matching_delimiter(code, open_idx, "{", "}")
        if close_idx is None:
            continue
        spans.append(ClassSpan(
            start=match.start(),
            body_start=open_idx + 1,
            body_end=close_idx,
            is_template=template_header_before(headers, match.start(), code) is not None,
        ))
    return spans


def inside_class_body(pos: int, spans: Iterable[ClassSpan]) -> bool:
    return any(span.body_start <= pos < span.body_end for span in spans)


def is_type_template_parameter(param: str) -> bool:
    return bool(re.match(r"(?:class|typename)\s*(?:\.\.\.)?\s*(?:[A-Za-z_][A-Za-z0-9_]*)?(?:\s*=.*)?$", param))


def is_template_template_parameter(param: str) -> bool:
    return bool(re.match(r"template\s*<", param))


def is_pointer_or_reference_nttp(param: str) -> bool:
    declaration = before_top_level_equal(param)
    return bool(re.search(r"::\s*\*|\(\s*\*|\*|&", declaration))


def add_source_hit(hits: dict[str, FeatureHit], feature_id: str, evidence: str) -> None:
    hit = hits.setdefault(feature_id, FeatureHit(feature_id))
    if evidence not in hit.evidence:
        hit.evidence.append(evidence)


def detect_structured_template_features(code: str) -> dict[str, FeatureHit]:
    hits: dict[str, FeatureHit] = {}
    headers = find_template_headers(code)
    spans = find_class_spans(code, headers)
    for header in headers:
        for param in header.params:
            if "..." in param:
                add_source_hit(hits, "template.pack", f"source:template parameter pack `{param}`")
            if has_top_level_equal(param):
                add_source_hit(hits, "template.default_argument", f"source:template parameter default `{param}`")
            if is_template_template_parameter(param):
                add_source_hit(hits, "template.template_parameter", f"source:template-template parameter `{param}`")
                continue
            if is_type_template_parameter(param):
                continue
            add_source_hit(hits, "template.nttp", f"source:non-type template parameter `{param}`")
            if is_pointer_or_reference_nttp(param):
                add_source_hit(hits, "template.nttp.pointer_member", f"source:pointer/reference NTTP `{param}`")
        if inside_class_body(header.start, spans):
            add_source_hit(hits, "template.member_template", "source:template declaration inside class body")
    for span in spans:
        if span.is_template:
            body = code[span.body_start:span.body_end]
            if re.search(r"\bfriend\b", body):
                add_source_hit(hits, "template.friend", "source:friend declaration inside class template")
    if re.search(r"\bsizeof\s*\.\.\.", code):
        add_source_hit(hits, "template.pack", "source:sizeof... pack expression")
    return hits


def detect_lambda_capture_features(code: str) -> dict[str, FeatureHit]:
    hits: dict[str, FeatureHit] = {}
    lambda_pattern = re.compile(
        r"(?<!operator)\[([^]\n]*)\]\s*\([^)]*\)\s*"
        r"(?:mutable\s*)?(?:noexcept\s*)?(?:->[^{]+)?\{",
        re.MULTILINE | re.DOTALL,
    )
    for match in lambda_pattern.finditer(code):
        capture_list = match.group(1).strip()
        if not capture_list:
            continue
        captures = split_top_level_commas(capture_list)
        has_ref_or_this_capture = False
        has_copy_or_default_capture = False
        for raw_capture in captures:
            capture = raw_capture.strip()
            if not capture:
                continue
            if capture == "=" or capture.startswith("=,"):
                has_copy_or_default_capture = True
                continue
            if capture == "&" or capture.startswith("&,") or capture == "this" or capture.startswith("&"):
                has_ref_or_this_capture = True
                continue
            has_copy_or_default_capture = True
        evidence = " ".join(match.group(0).split())[:120]
        if has_ref_or_this_capture:
            add_source_hit(
                hits,
                "support.lambda.capture.ref_this",
                f"source:{evidence}",
            )
        if has_copy_or_default_capture:
            add_source_hit(
                hits,
                "support.lambda.capture",
                f"source:{evidence}",
            )
    return hits


def evidence_partition(evidence: Iterable[str]) -> tuple[list[str], list[str]]:
    source_ref: list[str] = []
    path: list[str] = []
    for item in evidence:
        if item.startswith("path:"):
            path.append(item)
        else:
            source_ref.append(item)
    return source_ref, path


def inside_function_body(code: str, position: int) -> bool:
    """Return whether position is inside a function rather than a class body."""
    contexts: list[str] = []
    segment_start = 0
    for index, character in enumerate(code[:position]):
        if character == "{":
            prefix = code[segment_start:index].strip()
            parent = contexts[-1] if contexts else "namespace"
            if re.search(r"\b(?:class|struct|union)\b[^;{}]*$", prefix):
                context = "class"
            elif (
                re.search(r"\)\s*(?:const\s*)?(?:noexcept(?:\s*\([^)]*\))?\s*)?$", prefix)
                and not re.search(r"\b(?:if|for|while|switch|catch)\s*\([^{};]*\)\s*$", prefix)
            ):
                context = "function"
            else:
                context = parent
            contexts.append(context)
            segment_start = index + 1
        elif character == "}":
            if contexts:
                contexts.pop()
            segment_start = index + 1
        elif character == ";":
            segment_start = index + 1
    return bool(contexts and contexts[-1] == "function")


def has_function_local_dynamic_class_static(code: str) -> bool:
    return any(
        inside_function_body(code, match.start())
        for match in DYNAMIC_CLASS_STATIC_RE.finditer(code)
    )


def detect_features(source: str, ref_text: str = "", test_path: str = "") -> dict[str, FeatureHit]:
    if "/pa9/tests/abi/" in test_path or test_path.startswith("pa9/tests/abi/"):
        # PA9 consumes normalized ABI facts, not C++ source or LowIR. Words
        # such as `operator`, `virtual`, and `typeinfo` describe the encoder's
        # future-facing vocabulary and do not exercise the later language
        # features that use those encodings.
        return {}
    no_comments = strip_comments(source)
    code = strip_string_literals(no_comments)
    exception_code = strip_dynamic_exception_specifications(code)
    declared_intrinsics = declared_intrinsic_like_names(code)
    hits: dict[str, FeatureHit] = {}
    if exception_code != code:
        hits["function.dynamic_exception_spec"] = FeatureHit(
            "function.dynamic_exception_spec", ["source:dynamic exception specification"])
    for rule in RULES:
        haystack = no_comments if rule.use_raw else code
        if rule.feature_id == "exception.try_catch":
            haystack = exception_code
        if rule.feature_id == "template.builtin_traits":
            matched = match_builtin_trait_patterns(
                rule.patterns,
                strip_preprocessor_directives(haystack),
                declared_intrinsics,
            )
        else:
            if rule.feature_id == "template.dependent_name":
                matched = match_dependent_name_patterns(rule.patterns, haystack)
            else:
                matched = match_rule_patterns(rule.patterns, haystack, rule.all_patterns, "source")
            matched.extend(match_rule_patterns(rule.ref_patterns, ref_text, rule.all_patterns, "ref"))
            matched.extend(match_rule_patterns(rule.path_patterns, test_path, rule.all_patterns, "path"))
        if rule.feature_id == "class.member_pointer" and overloaded_arrow_star_without_member_pointer(code):
            matched = [evidence for evidence in matched if "->*" not in evidence]
        if (
            rule.feature_id == "lowir.procedural.local_static.dynamic_class"
            and not has_function_local_dynamic_class_static(code)
        ):
            matched = [evidence for evidence in matched if not evidence.startswith("source:")]
        if (
            rule.feature_id == "rtti.dynamic_cast.multi_vptr"
            and not re.search(r"\b(?:dynamic_cast|typeid)\s*(?:<|\()", code)
        ):
            # __vmi_class_type_info describes any polymorphic multiple-
            # inheritance RTTI object.  It is not by itself evidence that the
            # test exercises PA23 dynamic_cast/typeid behavior.
            matched = [evidence for evidence in matched if not evidence.startswith("ref:")]
        if (
            rule.feature_id == "class.inheritance.multiple"
            and has_materialized_multiple_inheritance(code)
        ):
            matched.append("source:<multiple base-specifiers>")
        if matched:
            hits[rule.feature_id] = FeatureHit(rule.feature_id, matched)
    for feature_id, structured_hit in detect_structured_template_features(code).items():
        hit = hits.setdefault(feature_id, FeatureHit(feature_id))
        for evidence in structured_hit.evidence:
            if evidence not in hit.evidence:
                hit.evidence.append(evidence)
    for feature_id, lambda_hit in detect_lambda_capture_features(code).items():
        hit = hits.setdefault(feature_id, FeatureHit(feature_id))
        for evidence in lambda_hit.evidence:
            if evidence not in hit.evidence:
                hit.evidence.append(evidence)
    host_object_attributes = re.findall(
        r"__attribute__\s*\(\(\s*([A-Za-z_][A-Za-z0-9_]*)",
        code,
    )
    if host_object_attributes:
        host_object_attribute_names = set(host_object_attributes)
        no_inline = host_object_attribute_names & {"noinline", "__noinline__"}
        if no_inline:
            hits["function.no_inline"] = FeatureHit(
                "function.no_inline",
                [f"source:__attribute__(({name}))" for name in sorted(no_inline)],
            )
        recognized_names = (
            PA27_HOST_OBJECT_ATTRIBUTE_NAMES
            if test_path.startswith("pa27/tests/")
            else HOST_OBJECT_ATTRIBUTE_NAMES
        )
        recognized = sorted(host_object_attribute_names & recognized_names)
        if recognized:
            hits["host.object_attribute"] = FeatureHit(
                "host.object_attribute",
                [f"source:__attribute__(({name}))" for name in recognized],
            )
        effect_names = {"pure", "const", "__pure__", "__const__"}
        effects = sorted(host_object_attribute_names & effect_names)
        if effects:
            hits["host.function_effect_attribute"] = FeatureHit(
                "host.function_effect_attribute",
                [f"source:__attribute__(({name}))" for name in effects],
            )
        if host_object_attribute_names <= (recognized_names | effect_names | no_inline):
            hits.pop("support.attribute", None)
        if (
            test_path.startswith("pa27/tests/")
            and "section" in host_object_attribute_names
        ):
            predefined = hits.get("support.host_predefined_macro")
            if predefined and all("__APPLE__" in item for item in predefined.evidence):
                hits.pop("support.host_predefined_macro", None)
    if (
        test_path.startswith("pa28/tests/")
        and re.search(r"\b(?:abi_tag|__abi_tag__?)\s*\(", code)
    ):
        hits["host.abi_name_attribute"] = FeatureHit(
            "host.abi_name_attribute",
            ["source:ABI tag on inspected host symbol"],
        )
        hits.pop("support.attribute", None)
    if (
        test_path.startswith("pa28/tests/")
        and re.search(r"(?:abi|mangl)", test_path)
        and re.search(r"\b__(?:decay|remove|add|underlying_type)\b", code)
    ):
        hits["host.abi_builtin_type"] = FeatureHit(
            "host.abi_builtin_type",
            ["source:builtin transform in inspected host ABI name"],
        )
        hits.pop("template.builtin_traits", None)
    if test_path.startswith("pa29/tests/run/"):
        hits["hosted.runtime_compat"] = FeatureHit(
            "hosted.runtime_compat",
            ["harness:PA29 hosted link/run"],
        )
    if (
        re.search(r"__local_static__|local_static_(?:init|ready)", ref_text)
        and LOWIR_EH_CONTROL_RE.search(ref_text)
    ):
        hits["exception.guarded_static_cleanup"] = FeatureHit(
            "exception.guarded_static_cleanup",
            ["source:function-local dynamic class static", "ref:EH control"],
        )
    if (
        re.search(r"\breturn\s*\{", code)
        and re.search(r"~[A-Za-z_][A-Za-z0-9_]*\s*\(", code)
        and LOWIR_EH_CONTROL_RE.search(ref_text)
    ):
        hits["exception.aggregate_cleanup"] = FeatureHit(
            "exception.aggregate_cleanup",
            ["source:aggregate return with destructor", "ref:EH control"],
        )
    destructible_classes = re.findall(
        r"~([A-Za-z_][A-Za-z0-9_]*)\s*\(",
        code,
    )
    class_value_parameter = next(
        (
            class_name
            for class_name in destructible_classes
            if re.search(
                rf"\(\s*{re.escape(class_name)}\s+[A-Za-z_][A-Za-z0-9_]*\s*\)",
                code,
            )
        ),
        None,
    )
    if (
        class_value_parameter
        and re.search(r"pass=by_address", ref_text)
        and LOWIR_EH_CONTROL_RE.search(ref_text)
    ):
        hits["exception.class_value_argument_cleanup"] = FeatureHit(
            "exception.class_value_argument_cleanup",
            [
                f"source:{class_value_parameter} value parameter with destructor",
                "ref:indirect class argument with EH control",
            ],
        )
    if (
        re.search(r"\bthrow\s+[A-Za-z_][A-Za-z0-9_]*\s*\(", code)
        and re.search(r"~[A-Za-z_][A-Za-z0-9_]*\s*\(", code)
        and re.search(r"role=eh_throw", ref_text)
        and LOWIR_EH_CONTROL_RE.search(ref_text)
    ):
        hits["exception.throw_operand_cleanup"] = FeatureHit(
            "exception.throw_operand_cleanup",
            ["source:class throw operand with destructor", "ref:EH throw control"],
        )
    if (
        re.search(r"\bcatch\s*\([^)]*\)\s*\{[^{}]*\bif\s*\(", code)
        and re.search(r"~[A-Za-z_][A-Za-z0-9_]*\s*\(", code)
        and re.search(r"^\s*eh_cleanup\b", ref_text, re.MULTILINE)
        and LOWIR_EH_CONTROL_RE.search(ref_text)
    ):
        hits["exception.handler_branch_cleanup"] = FeatureHit(
            "exception.handler_branch_cleanup",
            ["source:branched typed handler with outer destructor", "ref:EH cleanup control"],
        )
    return hits


UNJUDGED_LANE_DIRS = frozenset({"regression"})


def in_unjudged_lane(path: Path) -> bool:
    """Solution regression lanes are outside the student course contract."""
    return any(part in UNJUDGED_LANE_DIRS for part in path.parts)


def suite_test_files(test_root: Path) -> list[Path]:
    inputs = list(test_root.rglob("*.t"))
    control_root = test_root if test_root.name == "controls" else test_root / "controls"
    inputs.extend(control_root.rglob("*.cpp"))
    return [
        path for path in sorted(inputs)
        if path.is_file() and not in_unjudged_lane(path.relative_to(test_root))
    ]


def iter_test_files(root: Path, pas: Iterable[str]) -> list[Path]:
    files: list[Path] = []
    for pa in pas:
        test_root = root / pa / "tests"
        if test_root.exists():
            # Debug-info lanes exercise preservation through completed later
            # tools. They do not add those tools' source features to the local
            # assignment contract. Keep them in the separate hygiene scan.
            files.extend(path for path in suite_test_files(test_root)
                         if "debuginfo" not in path.relative_to(test_root).parts)
    return files


def iter_local_hygiene_pas(pas: Iterable[str]) -> list[str]:
    selected: list[str] = []
    for pa in pas:
        if pa_number(pa) is not None:
            selected.append(pa)
    return selected


def iter_local_test_files(root: Path, pas: Iterable[str]) -> list[Path]:
    files: list[Path] = []
    for pa in iter_local_hygiene_pas(pas):
        test_root = root / pa / "tests"
        if test_root.exists():
            files.extend(suite_test_files(test_root))
    return files


def is_hygiene_source_file(path: Path) -> bool:
    if re.search(r"\.t\.\d+$", path.name):
        return True
    return path.suffix in {
        ".t",
        ".h",
        ".hh",
        ".hpp",
        ".cc",
        ".cpp",
        ".cxx",
    }


def iter_local_hygiene_source_files(root: Path, pas: Iterable[str]) -> list[Path]:
    files: list[Path] = []
    for pa in iter_local_hygiene_pas(pas):
        test_root = root / pa / "tests"
        if not test_root.exists():
            continue
        for path in sorted(test_root.rglob("*")):
            if path.is_file() and is_hygiene_source_file(path):
                files.append(path)
    return files


def hosted_stl_include(header: str) -> bool:
    header = header.strip()
    if header in HOST_ABI_EH_COMPONENT_HEADERS:
        return False
    if header.startswith(HOSTED_STL_INTERNAL_INCLUDE_PREFIXES):
        return True
    return header in HOSTED_STL_HEADERS


def host_abi_eh_component_include(header: str) -> bool:
    return header.strip() in HOST_ABI_EH_COMPONENT_HEADERS


def hosted_eh_rtti_include(header: str) -> bool:
    return header.strip() in HOSTED_EH_RTTI_HEADERS


def test_local_include_override(root: Path, source_path: Path, header: str) -> bool:
    """Return whether a family-owned -I directory supplies this angle header."""
    family_name = re.sub(r"\.t(?:\.\d+)?$", "", source_path.name)
    flags_path = source_path.with_name(f"{family_name}.flags")
    if not flags_path.exists():
        return False
    try:
        tokens = shlex.split(read_text(flags_path))
    except ValueError:
        return False
    include_dirs: list[str] = []
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token == "-I" and index + 1 < len(tokens):
            include_dirs.append(tokens[index + 1])
            index += 2
            continue
        if token.startswith("-I") and len(token) > 2:
            include_dirs.append(token[2:])
        index += 1
    relative = source_path.relative_to(root)
    current_pa = current_pa_for(relative)
    pa_root = root / current_pa
    for include_dir in include_dirs:
        include_path = Path(include_dir)
        candidates = [include_path] if include_path.is_absolute() else [
            root / include_path,
            pa_root / include_path,
            source_path.parent / include_path,
        ]
        if any((candidate / header).is_file() for candidate in candidates):
            return True
    return False


# LowIR side-effect checks intentionally avoid broad object-name evidence such
# as object=_Z...; PA8 treats those names as presentation/backend metadata.
LOWIR_POLYMORPHIC_RE = re.compile(r"__vtable|_ZTV|__rtti|_ZTI|__typeinfo_name|_ZTS|typeinfo")
LOWIR_CLEANUP_EH_RE = re.compile(
    r"\beh_(?:try|cleanup|end)\b|\b_Unwind_Resume\b|"
    r"\b__gxx_personality_v0\b|\brole=eh_(?:resume|personality)\b|"
    r"\bresume\b|\bexception_selector\b|\b__cxa_(?:throw|begin_catch|end_catch|rethrow)\b"
)
LOWIR_EH_CONTROL_RE = re.compile(
    r"^\s*(?:eh_try|eh_cleanup|eh_end|resume|throw)\b|"
    r"^\s*%[A-Za-z0-9_]+\s*=\s*exception\b|"
    r"\bexception_selector\b",
    re.MULTILINE,
)
LOWIR_EH_RUNTIME_DECL_RE = re.compile(
    r"\brole=eh_(?:resume|personality)\b|"
    r"\b_Unwind_Resume\b|"
    r"\b__gxx_personality_v0\b"
)
SOURCE_EXCEPTION_RE = re.compile(r"\b(?:try|catch|throw)\b")
HOSTED_EXCEPTION_RUNTIME_RE = re.compile(
    r"\bstd\s*::\s*(?:exception_ptr|make_exception_ptr|rethrow_exception)\b"
)


def polymorphic_cleanup_lowir_evidence(source: str, ref_text: str) -> str:
    has_polymorphic_source = re.search(r"\bvirtual\b", strip_string_literals(strip_comments(source)))
    polymorphic_ref = LOWIR_POLYMORPHIC_RE.search(ref_text)
    cleanup_ref = LOWIR_CLEANUP_EH_RE.search(ref_text)
    if (has_polymorphic_source or polymorphic_ref) and cleanup_ref:
        evidence = []
        if has_polymorphic_source:
            evidence.append("source:virtual")
        if polymorphic_ref:
            evidence.append(f"ref:{polymorphic_ref.group(0)}")
        evidence.append(f"ref:{cleanup_ref.group(0)}")
        return ", ".join(evidence)
    return ""


def hidden_eh_lowir_evidence(source: str, ref_text: str) -> str:
    stripped_source = strip_dynamic_exception_specifications(
        strip_string_literals(strip_comments(source)))
    if SOURCE_EXCEPTION_RE.search(stripped_source):
        return ""
    cleanup_ref = LOWIR_CLEANUP_EH_RE.search(ref_text)
    if cleanup_ref:
        return f"ref:{cleanup_ref.group(0)}"
    return ""


def lowir_eh_evidence(match: re.Match[str] | None) -> str:
    if not match:
        return ""
    return " ".join(match.group(0).split())[:120]


def scan_lowir_eh_review(root: Path, pas: Iterable[str]) -> list[LowIREHFinding]:
    findings: list[LowIREHFinding] = []
    for path in iter_local_test_files(root, pas):
        relative_path = path.relative_to(root)
        current_pa = current_pa_for(relative_path)
        if not is_lowir_test(current_pa):
            continue
        current_number = pa_number(current_pa)
        if current_number is not None and current_number >= SOURCE_EH_LOWIR_OWNER_PA:
            continue
        source = read_text(path) + "\n" + companion_source_text_for(path)
        stripped_source = strip_dynamic_exception_specifications(
            strip_string_literals(strip_comments(source)))
        if SOURCE_EXCEPTION_RE.search(stripped_source):
            continue
        ref_text = ref_text_for(path)
        if not ref_text:
            continue
        control = LOWIR_EH_CONTROL_RE.search(ref_text)
        runtime = LOWIR_EH_RUNTIME_DECL_RE.search(ref_text)
        if not control and not runtime:
            continue
        evidence_parts = []
        control_evidence = lowir_eh_evidence(control)
        runtime_evidence = lowir_eh_evidence(runtime)
        if control_evidence:
            evidence_parts.append(f"ref:{control_evidence}")
        if runtime_evidence:
            evidence_parts.append(f"ref:{runtime_evidence}")
        findings.append(LowIREHFinding(
            path=relative_path.as_posix(),
            kind="eh-control" if control else "eh-runtime-declaration-only",
            evidence=", ".join(evidence_parts),
        ))
    return findings


def scan_generated_lowir_eh_review(
    root: Path,
    compiler: Path,
    tests: Iterable[Path],
    generated_lowir_by_test: dict[str, str] | None = None,
) -> list[LowIREHFinding]:
    """Probe selected host tests whose checked oracle does not contain LowIR."""
    findings: list[LowIREHFinding] = []
    cache: dict[Path, tuple[int, str, str]] = {}
    with tempfile.TemporaryDirectory(prefix="cppgm-placement-lowir.") as temp_dir:
        output_root = Path(temp_dir)
        for path in tests:
            relative_path = path.relative_to(root).as_posix()
            sources = numbered_test_source_files_for(path)
            if not sources:
                findings.append(LowIREHFinding(
                    path=relative_path,
                    kind="lowir-probe-failed",
                    evidence="no numbered .t.N source companions",
                ))
                continue
            for index, source in enumerate(sources):
                cached = cache.get(source)
                if cached is None:
                    output = output_root / f"probe-{len(cache)}-{index}.lowir"
                    try:
                        completed = subprocess.run(
                            [
                                str(compiler),
                                "--emit-lowir",
                                "-O0",
                                "-o",
                                str(output),
                                str(source),
                            ],
                            cwd=root,
                            capture_output=True,
                            text=True,
                            timeout=60,
                            check=False,
                        )
                        lowir = read_text(output) if completed.returncode == 0 and output.exists() else ""
                        cached = (completed.returncode, lowir, completed.stderr)
                    except (OSError, subprocess.TimeoutExpired) as error:
                        cached = (1, "", str(error))
                    cache[source] = cached
                returncode, lowir, stderr = cached
                source_relative = source.relative_to(root).as_posix()
                if returncode != 0:
                    detail = " ".join(stderr.split())[:160]
                    evidence = f"generated:{source_relative}:exit={returncode}"
                    if detail:
                        evidence += f":{detail}"
                    findings.append(LowIREHFinding(
                        path=relative_path,
                        kind="lowir-probe-failed",
                        evidence=evidence,
                    ))
                    continue
                if generated_lowir_by_test is not None:
                    existing = generated_lowir_by_test.get(relative_path, "")
                    generated_lowir_by_test[relative_path] = (
                        lowir if not existing else f"{existing}\n{lowir}"
                    )
                control = LOWIR_EH_CONTROL_RE.search(lowir)
                runtime = LOWIR_EH_RUNTIME_DECL_RE.search(lowir)
                if not control and not runtime:
                    continue
                evidence_parts = []
                control_evidence = lowir_eh_evidence(control)
                runtime_evidence = lowir_eh_evidence(runtime)
                if control_evidence:
                    evidence_parts.append(f"generated:{source_relative}:{control_evidence}")
                if runtime_evidence:
                    evidence_parts.append(f"generated:{source_relative}:{runtime_evidence}")
                findings.append(LowIREHFinding(
                    path=relative_path,
                    kind=(
                        "generated-eh-control"
                        if control
                        else "generated-eh-runtime-declaration-only"
                    ),
                    evidence=", ".join(evidence_parts),
                ))
    return findings


def generated_lowir_feature_hits(
    source: str,
    lowir: str,
    test_path: str,
) -> dict[str, FeatureHit]:
    """Run checked-reference feature rules over an explicitly probed LowIR unit."""
    result: dict[str, FeatureHit] = {}
    for feature_id, hit in detect_features(source, lowir, test_path).items():
        if not any(evidence.startswith("ref:") for evidence in hit.evidence):
            continue
        result[feature_id] = FeatureHit(
            feature_id,
            [
                evidence.replace("ref:", "generated:", 1)
                if evidence.startswith("ref:")
                else evidence
                for evidence in hit.evidence
            ],
        )
    return result


def pa20_lowir_side_effect_findings(path: Path, source: str, ref_text: str) -> list[HygieneFinding]:
    relative = path.as_posix()
    findings: list[HygieneFinding] = []
    evidence = polymorphic_cleanup_lowir_evidence(source, ref_text)
    if evidence:
        findings.append(HygieneFinding(
            path=relative,
            kind="pa20-rtti-eh-side-effect",
            message=(
                "PA20 tests should split or move polymorphic RTTI/typeinfo "
                "output that also needs EH/unwind lowering"
            ),
            evidence=evidence,
        ))
        return findings
    evidence = hidden_eh_lowir_evidence(source, ref_text)
    if evidence:
        findings.append(HygieneFinding(
            path=relative,
            kind="pa20-hidden-eh-side-effect",
            message=(
                "PA20 tests should split or move source constructs that emit "
                "EH/unwind lowering without explicit exception syntax"
            ),
            evidence=evidence,
        ))
    return findings


def scan_test_hygiene(root: Path, pas: Iterable[str]) -> list[HygieneFinding]:
    findings: list[HygieneFinding] = []
    selected_pas = tuple(pas)
    for pa in iter_local_hygiene_pas(selected_pas):
        test_root = root / pa / "tests"
        if not test_root.exists():
            continue
        for path in sorted(test_root.rglob("*.compile.flags")):
            findings.append(HygieneFinding(
                path=path.relative_to(root).as_posix(),
                kind="compile-flags-sidecar",
                message=(
                    "per-test .compile.flags sidecars are not allowed; add a "
                    "dedicated harness sidecar for the specific behavior being "
                    "tested"
                ),
            ))
    for path in iter_local_test_files(root, selected_pas):
        relative = path.relative_to(root).as_posix()
        cluster = cluster_for(path)
        pa_index = pa_number(current_pa_for(path.relative_to(root)))
        if cluster is None:
            findings.append(HygieneFinding(
                path=relative,
                kind="test-number",
                message=(
                    "tests must start with a three-digit prefix"
                    if pa_index is not None and pa_index < 5 else
                    "local PA5+ tests must start with a three-digit cluster prefix"
                ),
            ))
        elif pa_index is not None and pa_index < 5:
            pass
        elif cluster not in VALID_TEST_CLUSTERS:
            nearest = (cluster // 100) * 100
            findings.append(HygieneFinding(
                path=relative,
                kind="test-number",
                message=(
                    f"test prefix {cluster:03d} is an individual number, not a "
                    f"cluster; use {nearest:03d} for this cluster"
                ),
            ))
        current_pa = current_pa_for(path.relative_to(root))
        if current_pa == "pa18":
            source = read_text(path) + "\n" + companion_source_text_for(path)
            evidence = polymorphic_cleanup_lowir_evidence(source, ref_text_for(path))
            if evidence:
                findings.append(HygieneFinding(
                    path=relative,
                    kind="pa18-heavy-support",
                    message=(
                        "PA18 template tests should split or move polymorphic "
                        "RTTI/vtable output that also needs cleanup-unwind lowering"
                    ),
                    evidence=evidence,
                ))
        if current_pa == "pa20":
            source = read_text(path) + "\n" + companion_source_text_for(path)
            findings.extend(pa20_lowir_side_effect_findings(
                path.relative_to(root),
                source,
                ref_text_for(path),
            ))
    include_re = re.compile(r"^\s*#\s*include\s*<([^>\n]+)>", re.MULTILINE)
    for path in iter_local_hygiene_source_files(root, selected_pas):
        current_pa = current_pa_for(path.relative_to(root))
        number = pa_number(current_pa)
        if number is None:
            continue
        relative = path.relative_to(root).as_posix()
        source = read_text(path)
        if number <= ABI_NAMING_EARLY_PA_MAX:
            path_match = ABI_NAMING_WORD_RE.search(relative)
            source_match = ABI_NAMING_WORD_RE.search(source)
            if path_match or source_match:
                evidence = (
                    f"path:{path_match.group(0)}" if path_match
                    else f"source:{source_match.group(0)}"
                )
                findings.append(HygieneFinding(
                    path=relative,
                    kind="early-abi-naming-wording",
                    message=(
                        f"ABI naming/mangling terminology belongs in "
                        f"{ABI_NAMING_OWNER_PA} or later; earlier tests "
                        "should describe the source-language behavior instead"
                    ),
                    evidence=evidence,
                ))
        if number <= HOSTED_EXCEPTION_RUNTIME_EARLY_PA_MAX:
            match = HOSTED_EXCEPTION_RUNTIME_RE.search(source)
            if match:
                findings.append(HygieneFinding(
                    path=relative,
                    kind="early-hosted-exception-runtime",
                    message=(
                        "hosted std::exception_ptr/make_exception_ptr/"
                        f"rethrow_exception runtime behavior belongs in "
                        f"{HOSTED_EXCEPTION_RUNTIME_OWNER_PA} or later"
                    ),
                    evidence=match.group(0),
                ))
        for match in include_re.finditer(source):
            header = match.group(1).strip()
            if test_local_include_override(root, path, header):
                continue
            if number <= HOSTED_EH_RTTI_HEADER_EARLY_PA_MAX and hosted_eh_rtti_include(header):
                findings.append(HygieneFinding(
                    path=relative,
                    kind="early-hosted-eh-rtti-header",
                    message=(
                        f"hosted EH/RTTI header <{header}> belongs in "
                        f"{HOSTED_EH_RTTI_HEADER_OWNER_PA} or later"
                    ),
                    evidence=f"#include <{header}>",
                ))
            elif host_abi_eh_component_include(header):
                findings.append(HygieneFinding(
                    path=relative,
                    kind="early-host-abi-eh-header",
                    message=(
                        f"host ABI/EH component header <{header}> is not "
                        "allowed before its owner stage"
                    ),
                    evidence=f"#include <{header}>",
                ))
            elif number <= HOSTED_STL_EARLY_PA_MAX and hosted_stl_include(header):
                findings.append(HygieneFinding(
                    path=relative,
                    kind="early-hosted-stl",
                    message=(
                        f"hosted STL include <{header}> belongs in "
                        f"{HOSTED_STL_OWNER_PA} or later"
                    ),
                    evidence=f"#include <{header}>",
                ))
    return findings


def current_pa_for(path: Path) -> str:
    parts = path.parts
    for part in parts:
        if re.fullmatch(r"pa\d+", part):
            return part
    return "unknown"


def test_role_for(path: Path) -> str:
    parts = path.parts
    if "controls" in parts:
        return "control"
    if "general" in parts:
        return "general"
    if "spec" in parts:
        return "spec"
    return "other"


def cluster_for(path: Path) -> int | None:
    match = re.match(r"(\d{3})-", path.name)
    return int(match.group(1)) if match else None


def expected_exit_for(path: Path) -> str:
    ref_status = path.with_suffix(".ref.exit_status")
    if ref_status.exists():
        return read_text(ref_status).strip()
    return "unknown"


def ref_text_for(path: Path) -> str:
    ref = path.with_suffix(".ref")
    if ref.exists():
        return read_text(ref)
    return ""


def numbered_test_source_files_for(path: Path) -> list[Path]:
    prefix = f"{path.name}."
    return [
        sidecar
        for sidecar in sorted(path.parent.glob(f"{path.name}.*"))
        if sidecar.is_file()
        and sidecar.name.startswith(prefix)
        and sidecar.name[len(prefix):].isdigit()
    ]


def companion_source_text_for(path: Path) -> str:
    """Return same-stem source sidecars that can carry the tested behavior."""
    sidecar_suffixes = (
        ".h",
        ".hh",
        ".hpp",
        ".shared.h",
        ".shared.hh",
        ".shared.hpp",
        ".cc",
        ".cpp",
        ".cxx",
    )
    numbered_sources = set(numbered_test_source_files_for(path))
    chunks: list[str] = []
    for sidecar in sorted(path.parent.glob(f"{path.stem}.*")):
        if sidecar == path or sidecar.suffix.startswith(".ref"):
            continue
        if (
            sidecar.name.endswith(sidecar_suffixes)
            or sidecar in numbered_sources
        ):
            chunks.append(read_text(sidecar))
    return "\n".join(chunks)


def host_eh_object_evidence(path: Path, current_pa: str, source: str) -> str:
    """Identify the PA26 host-object layer without relying on its filename."""
    if current_pa != "pa26" or not numbered_test_source_files_for(path):
        return ""
    stripped_source = strip_dynamic_exception_specifications(
        strip_string_literals(strip_comments(source)))
    source_match = SOURCE_EXCEPTION_RE.search(stripped_source)
    if source_match:
        return f"harness:cppgm++ -c, source:{source_match.group(0)}"
    inspect_ref = path.with_suffix(".ref.inspect")
    if inspect_ref.exists():
        inspect_match = re.search(
            r"\b(?:lsda|host_(?:lsda|unwind)|__gxx_personality_v0|"
            r"_Unwind_Resume|__cxa_(?:throw|begin_catch|end_catch))\b",
            read_text(inspect_ref),
        )
        if inspect_match:
            return f"ref.inspect:{inspect_match.group(0)}"
    return ""


def host_object_interop_evidence(path: Path, current_pa: str) -> str:
    """Identify an asserted PA27 host-object layer without anchoring every test."""
    if current_pa != "pa27" or not numbered_test_source_files_for(path):
        return ""
    anchor = read_text(path)
    marker = re.search(r"\b(?:host-object|compiler-object)\b", anchor, re.IGNORECASE)
    if marker:
        return f"anchor-contract:{marker.group(0).lower()}"
    base_name = path.with_suffix("").name
    inspect_sidecars = sorted(path.parent.glob(f"{base_name}.inspect.*"))
    if inspect_sidecars:
        return f"harness:{inspect_sidecars[0].name}"
    if path.with_suffix(".ref.inspect").exists():
        return f"harness:{path.with_suffix('.ref.inspect').name}"
    return ""


def is_lowir_test(pa: str) -> bool:
    number = pa_number(pa)
    return number in LOWIR_SOURCE_PAS if number is not None else False


def is_pre_lowir_semantic_surface(feature_id: str, current_pa: str) -> bool:
    return (
        feature_id in PRE_LOWIR_SEMANTIC_SURFACE_FEATURES
        or feature_id.startswith(PRE_LOWIR_SEMANTIC_SURFACE_PREFIXES)
        or (current_pa == "pa7" and feature_id in PA7_SEMANTIC_SURFACE_FEATURES)
    )


def placement_for(feature: FeatureMeta, current_pa: str, current_cluster: int | None) -> tuple[str, str]:
    current_num = pa_number(current_pa)
    owner_num = pa_number(feature.owner_pa)
    if current_num is None or owner_num is None:
        return "review", "non-PA path or owner"
    if current_pa == "pa5" and current_num < owner_num:
        return (
            "semantic-surface",
            "AST output records source syntax without exercising its later "
            "semantic or lowering owner",
        )
    if (
        PRE_LOWIR_SEMANTIC_PA_MIN <= current_num <= SEMANTIC_ONLY_PA_MAX
        and current_num < owner_num
        and is_pre_lowir_semantic_surface(feature.feature_id, current_pa)
    ):
        return (
            "semantic-surface",
            "pre-LowIR AST/type/call output does not exercise the later "
            "source-to-LowIR or runtime owner",
        )
    if current_num >= 10 and owner_num <= SEMANTIC_ONLY_PA_MAX:
        return (
            "semantic-owner",
            "semantic owner cannot own LowIR output; place by enclosing LowIR feature",
        )
    if owner_num in BACKEND_ONLY_PAS and is_lowir_test(current_pa):
        return (
            "backend-owner",
            "backend owner expects LowIR/backend input; source-to-LowIR owner must be chosen",
        )
    if current_num < owner_num:
        return "violation", f"feature owner {feature.owner_pa} is later than {current_pa}"
    if current_num == owner_num and current_cluster is not None and current_cluster < feature.owner_cluster:
        return (
            "cluster-early",
            f"feature first cluster {feature.owner_cluster} is later than {current_cluster}",
        )
    return "ok", "owner reached"


def template_concepts_for(detected_features: Iterable[str]) -> list[str]:
    return sorted({
        concept
        for feature_id in detected_features
        if (concept := TEMPLATE_CONCEPT_BY_FEATURE.get(feature_id))
    })


def review_template_concepts(concepts: Iterable[str], current_pa: str) -> list[str]:
    review = set(concepts)
    if "sfinae" in review:
        review.discard("substitution")
    if len(review) > 1:
        support = (
            TEMPLATE_INTEGRATION_BASIC_SUPPORT
            if current_pa == "pa19"
            else TEMPLATE_PRE_INTEGRATION_SUPPORT
        )
        non_support = review.difference(support)
        if non_support:
            review = non_support
        else:
            review.difference_update(TEMPLATE_INTEGRATION_BASIC_SUPPORT)
    return sorted(review)


def later_template_dependency_features(
    placements: Iterable[dict[str, object]],
    current_pa: str,
    current_cluster: int | None,
) -> list[str]:
    later: list[str] = []
    current_key = placement_key(current_pa, current_cluster)
    for placement in placements:
        feature_id = str(placement["feature"])
        if feature_id in TEMPLATE_LATER_OR_COMPAT_FEATURES:
            owner_key = placement_key(
                str(placement.get("owner_pa", "")),
                placement.get("owner_cluster"),  # type: ignore[arg-type]
            )
            if current_key is None or owner_key is None or owner_key > current_key:
                later.append(feature_id)
    return sorted(set(later))


def latest_owner_label(feature_ids: Iterable[str], features: dict[str, FeatureMeta]) -> str:
    owners: list[tuple[int, int, str]] = []
    for feature_id in feature_ids:
        meta = features.get(feature_id)
        if not meta:
            continue
        owner_num = pa_number(meta.owner_pa)
        if owner_num is None:
            continue
        owners.append((owner_num, meta.owner_cluster, meta.owner_pa))
    if not owners:
        return ""
    owner_num, owner_cluster, owner_pa = max(owners)
    return f"{owner_pa}:{owner_cluster}"


def placement_key(pa: str, cluster: int | None) -> tuple[int, int] | None:
    number = pa_number(pa)
    if number is None:
        return None
    return (number, cluster or 0)


def late_audit_feature(meta: FeatureMeta) -> bool:
    if meta.feature_id in LATE_PLACEMENT_IGNORED_FEATURES:
        return False
    owner_num = pa_number(meta.owner_pa)
    if owner_num is None:
        return False
    return owner_num > SEMANTIC_ONLY_PA_MAX


def late_placement_candidate(
    placements: list[dict[str, object]],
    current_pa: str,
    current_cluster: int | None,
    features: dict[str, FeatureMeta],
) -> dict[str, object] | None:
    current_key = placement_key(current_pa, current_cluster)
    if current_key is None:
        return None
    if any(
        str(placement["feature"]) in LATE_PLACEMENT_ANCHOR_FEATURES
        and placement.get("owner_pa") == current_pa
        for placement in placements
    ):
        return None
    owner_entries: list[tuple[tuple[int, int], str, FeatureMeta, dict[str, object]]] = []
    for placement in placements:
        feature_id = str(placement["feature"])
        meta = features.get(feature_id)
        if not meta or not late_audit_feature(meta):
            continue
        owner_key = placement_key(meta.owner_pa, meta.owner_cluster)
        if owner_key is None:
            continue
        owner_entries.append((owner_key, feature_id, meta, placement))
    if not owner_entries:
        return None
    latest_key = max(entry[0] for entry in owner_entries)
    if latest_key >= current_key:
        return None
    latest_entries = [entry for entry in owner_entries if entry[0] == latest_key]
    latest_meta = latest_entries[0][2]
    latest_features = sorted({feature_id for _, feature_id, _, _ in latest_entries})
    confidence = (
        "broad-owner-only"
        if all(feature_id in LATE_PLACEMENT_BROAD_FEATURES for feature_id in latest_features)
        else "specific"
    )
    evidence: list[str] = []
    for _, _, _, placement in latest_entries:
        evidence.extend(str(item) for item in placement.get("evidence", [])[:2])
    return {
        "status": "late-placement-candidate",
        "suggested_pa": latest_meta.owner_pa,
        "suggested_cluster": latest_meta.owner_cluster,
        "features": latest_features,
        "confidence": confidence,
        "reason": (
            f"latest source/ref-backed owner is {latest_meta.owner_pa}:"
            f"{latest_meta.owner_cluster}, earlier than {current_pa}:"
            f"{current_cluster}"
        ),
        "evidence": evidence[:4],
    }


def suggest_integration_cluster(concepts: Iterable[str], current_cluster: int | None) -> int:
    concept_set = set(concepts)
    if current_cluster is not None and current_cluster >= 500:
        return 500
    if concept_set & {"detector-idiom", "sfinae", "substitution", "no-eager-instantiation"}:
        return 300
    if concept_set & {
        "function-deduction",
        "function-partial-ordering",
        "specialization-partial-ordering",
        "non-deduced-context",
        "braced-init-deduction",
    }:
        return 200
    if concept_set & {
        "alias-template",
        "variable-template",
        "member-template",
        "pack-expansion",
        "template-template-parameter",
    }:
        return 400
    return 100


def template_review_for(
    detected_features: list[str],
    path_hint_features: Iterable[str],
    placements: list[dict[str, object]],
    current_cluster: int | None,
    current_pa: str,
    features: dict[str, FeatureMeta],
) -> dict[str, object]:
    review_features = sorted({
        feature_id
        for feature_id in [*detected_features, *path_hint_features]
        if feature_id in TEMPLATE_CONCEPT_BY_FEATURE
    })
    effective_review_features = review_features
    if current_pa == "pa18":
        pa18_owned_features = [
            feature_id for feature_id in review_features
            if (meta := features.get(feature_id))
            and (pa_number(meta.owner_pa) or 0) >= 18
        ]
        if pa18_owned_features:
            effective_review_features = sorted(pa18_owned_features)
    all_concepts = template_concepts_for(review_features)
    integration_concepts = review_template_concepts(all_concepts, current_pa)
    concepts = template_concepts_for(effective_review_features)
    review_concepts = review_template_concepts(concepts, current_pa)
    template_features = sorted(
        feature_id for feature_id in detected_features
        if feature_id in TEMPLATE_CONCEPT_BY_FEATURE
    )
    later_features = later_template_dependency_features(placements, current_pa, current_cluster)
    owner = latest_owner_label(effective_review_features, features)
    suggested_cluster: int | None = None
    if not review_concepts:
        bucket = "manual-review"
        action = "Classify by source/ref review; no template concept was detected."
    elif later_features:
        bucket = "later-owner-or-split"
        action = "Move later-owned behavior, or split/reduce to keep only the PA18 template assertion."
    elif len(review_concepts) >= 2:
        bucket = "pa19-integration-candidate"
        suggested_cluster = suggest_integration_cluster(review_concepts, current_cluster)
        action = "Review as multi-feature template integration; move to PA19 if concepts are essential together."
    elif owner.startswith(("pa14", "pa15", "pa17")):
        bucket = "basic-owner-candidate"
        action = "Place in the owning basic template PA; keep if already there, otherwise move or renumber after review."
    elif owner.startswith("pa18"):
        bucket = "pa18-advanced-single-candidate"
        action = "Place in PA18 and renumber if the current cluster is earlier than the owner cluster."
    else:
        bucket = "manual-review"
        action = "Review manually; ownership is not resolved by the template classifier."
    return {
        "template_features": template_features,
        "template_concepts": concepts,
        "review_template_concepts": review_concepts,
        "template_concept_arity": len(review_concepts),
        # PA18 narrows the ordinary ownership view to PA18-owned
        # features so a single deduction/SFINAE assertion is not displaced by
        # its prerequisite syntax.  Preserve a second, unfiltered view for the
        # separate question of whether several completed template mechanisms
        # may be essential together and therefore belong in PA19 integration.
        "integration_template_concepts": integration_concepts,
        "integration_template_concept_arity": len(integration_concepts),
        "later_or_compat_features": later_features,
        "latest_template_owner": owner,
        "template_bucket": bucket,
        "suggested_pa18_cluster": suggested_cluster,
        "template_action": action,
    }


def row_for(path: Path,
            root: Path,
            features: dict[str, FeatureMeta]) -> dict[str, object]:
    source = read_text(path)
    sidecar_source = companion_source_text_for(path)
    detection_source = source if not sidecar_source else f"{source}\n{sidecar_source}"
    ref_text = ref_text_for(path)
    relative_path = path.relative_to(root)
    current_pa = current_pa_for(relative_path)
    current_cluster = cluster_for(path)
    # Token processing can contain arbitrary C++-looking text. Introductory
    # LowIR fixtures describe IR records, not the C++ constructs that later
    # lower to them. Their contracts are checked by their own suites and the
    # LowIR contract audit; C++ source detectors do not judge these inputs.
    current_number = pa_number(current_pa)
    input_contract = (
        "preprocessing" if current_number in {1, 2, 3, 4} else
        "lowir-introduction" if current_pa == "pa8" else
        "lowir-backend" if current_pa == "pa24" and path.suffix != ".cpp" else
        "abi-facts" if current_pa == "pa9" else
        "cpp-source"
    )
    raw_hits = (detect_features(detection_source, ref_text, relative_path.as_posix())
                if input_contract == "cpp-source" else {})
    host_eh_evidence = host_eh_object_evidence(path, current_pa, detection_source)
    if host_eh_evidence:
        raw_hits["host.eh_object"] = FeatureHit(
            "host.eh_object",
            [host_eh_evidence],
        )
    host_object_evidence = host_object_interop_evidence(path, current_pa)
    if host_object_evidence:
        raw_hits["host.object_interop"] = FeatureHit(
            "host.object_interop",
            [host_object_evidence],
        )
    hits: dict[str, FeatureHit] = {}
    path_hints: dict[str, list[str]] = {}
    for feature_id, hit in raw_hits.items():
        source_ref_evidence, path_evidence = evidence_partition(hit.evidence)
        if source_ref_evidence:
            hits[feature_id] = FeatureHit(feature_id, source_ref_evidence)
        if path_evidence:
            path_hints[feature_id] = path_evidence
    detected = sorted(hits)
    placements = []
    for feature_id in detected:
        meta = features.get(feature_id)
        if not meta:
            placements.append({
                "feature": feature_id,
                "status": "unknown-feature",
                "reason": "rule has no feature-table row",
                "owner_pa": "",
                "owner_cluster": None,
            })
            continue
        status, reason = placement_for(meta, current_pa, current_cluster)
        placements.append({
            "feature": feature_id,
            "status": status,
            "reason": reason,
            "owner_pa": meta.owner_pa,
            "owner_cluster": meta.owner_cluster,
            "evidence": hits[feature_id].evidence,
        })
    review_statuses = {"violation", "cluster-early", "backend-owner", "unknown-feature"}
    semantic_notes = [
        p
        for p in placements
        if p["status"] in {"semantic-owner", "semantic-surface"}
    ]
    late_candidate = late_placement_candidate(placements, current_pa, current_cluster, features)
    row = {
        "path": path.relative_to(root).as_posix(),
        "current_pa": current_pa,
        "current_cluster": current_cluster,
        "input_contract": input_contract,
        "test_role": test_role_for(path.relative_to(root)),
        "expected_exit": expected_exit_for(path),
        "detected_features": detected,
        "path_hint_features": sorted(path_hints),
        "path_hints": path_hints,
        "placements": placements,
        "needs_review": any(p["status"] in review_statuses for p in placements),
        "semantic_owner_notes": len(semantic_notes),
        "late_placement_candidate": late_candidate,
        "source_sidecars_scanned": bool(sidecar_source),
    }
    row.update(template_review_for(
        detected,
        sorted(path_hints),
        placements,
        current_cluster,
        current_pa,
        features,
    ))
    return row


def add_row_feature(
    row: dict[str, object],
    feature_id: str,
    evidence: str,
    features: dict[str, FeatureMeta],
) -> None:
    """Attach evidence discovered after the initial source/reference scan."""
    detected = list(row["detected_features"])  # type: ignore[arg-type]
    if feature_id in detected:
        return
    meta = features.get(feature_id)
    if meta is None:
        return
    current_pa = str(row["current_pa"])
    current_cluster = row["current_cluster"]
    status, reason = placement_for(meta, current_pa, current_cluster)  # type: ignore[arg-type]
    placements = list(row["placements"])  # type: ignore[arg-type]
    placements.append({
        "feature": feature_id,
        "status": status,
        "reason": reason,
        "owner_pa": meta.owner_pa,
        "owner_cluster": meta.owner_cluster,
        "evidence": [evidence],
    })
    detected.append(feature_id)
    row["detected_features"] = sorted(detected)
    row["placements"] = placements
    row["needs_review"] = bool(row["needs_review"]) or status in {
        "violation",
        "cluster-early",
        "backend-owner",
        "unknown-feature",
    }
    row["late_placement_candidate"] = late_placement_candidate(
        placements,
        current_pa,
        current_cluster,  # type: ignore[arg-type]
        features,
    )


def markdown_report(rows: list[dict[str, object]],
                    missing_rules: list[str],
                    include_ok: bool,
                    hygiene_findings: list[HygieneFinding] | None = None,
                    lowir_eh_findings: list[LowIREHFinding] | None = None) -> str:
    hygiene_findings = hygiene_findings or []
    lowir_eh_findings = lowir_eh_findings or []
    review_rows = [row for row in rows if row["needs_review"] or include_ok]
    late_rows = [row for row in rows if row.get("late_placement_candidate")]
    violations = sum(
        1
        for row in rows
        for placement in row["placements"]  # type: ignore[index]
        if placement["status"] in {"violation", "cluster-early", "backend-owner", "unknown-feature"}
    )
    semantic_notes = sum(int(row["semantic_owner_notes"]) for row in rows)
    path_hint_count = sum(len(row["path_hint_features"]) for row in rows)  # type: ignore[arg-type]
    lines = [
        "# PA Feature Placement Audit",
        "",
        f"- tests scanned: {len(rows)}",
        f"- tests needing review: {sum(1 for row in rows if row['needs_review'])}",
        f"- placement findings: {violations}",
        f"- semantic-owner notes: {semantic_notes}",
        f"- path-only hints: {path_hint_count}",
        f"- late placement candidates: {len(late_rows)}",
        f"- local test hygiene findings: {len(hygiene_findings)}",
        f"- LowIR EH review findings: {len(lowir_eh_findings)}",
        f"- feature table entries without detector rules: {len(missing_rules)}",
        "",
    ]
    if lowir_eh_findings:
        lines.append("## LowIR EH Review Findings")
        lines.append("")
        lines.append(
            "These are review leads for comparison-visible EH LowIR. The "
            "`eh-control` and runtime-only kinds come from checked LowIR refs; "
            "the `generated-*` kinds come only from explicitly requested "
            "compiler probes of host-test `.t.N` sources. The script reports "
            "probe failures because they provide no safe negative evidence. "
            "`unwind=` metadata is ignored."
        )
        lines.append("")
        lines.append("| Test | Kind | Evidence |")
        lines.append("| --- | --- | --- |")
        for finding in lowir_eh_findings:
            lines.append(
                "| `{}` | `{}` | `{}` |".format(
                    finding.path,
                    finding.kind,
                    finding.evidence.replace("|", "\\|"),
                )
            )
        lines.append("")
    if hygiene_findings:
        lines.append("## Local Test Hygiene Findings")
        lines.append("")
        lines.append("| Test | Kind | Reason | Evidence |")
        lines.append("| --- | --- | --- | --- |")
        for finding in hygiene_findings:
            lines.append(
                "| `{}` | `{}` | {} | `{}` |".format(
                    finding.path,
                    finding.kind,
                    markdown_cell(finding.message),
                    finding.evidence.replace("|", "\\|"),
                )
            )
        lines.append("")
    if missing_rules:
        lines.append("## Missing Detector Rules")
        lines.append("")
        for feature_id in missing_rules:
            lines.append(f"- `{feature_id}`")
        lines.append("")
    lines.append("## Placement Findings")
    lines.append("")
    lines.append("| Test | Current | Feature | Owner | Status | Reason | Evidence |")
    lines.append("| --- | --- | --- | --- | --- | --- | --- |")
    for row in review_rows:
        for placement in row["placements"]:  # type: ignore[index]
            if not include_ok and placement["status"] not in {
                "violation",
                "cluster-early",
                "backend-owner",
                "unknown-feature",
            }:
                continue
            current = f"{row['current_pa']}:{row['current_cluster']}"
            owner_cluster = placement.get("owner_cluster")
            owner = f"{placement.get('owner_pa')}:{owner_cluster}" if owner_cluster else str(placement.get("owner_pa"))
            evidence = ", ".join(placement.get("evidence", [])[:2])
            lines.append(
                "| `{}` | `{}` | `{}` | `{}` | `{}` | {} | `{}` |".format(
                    row["path"],
                    current,
                    placement["feature"],
                    owner,
                    placement["status"],
                    str(placement["reason"]).replace("|", "\\|"),
                    evidence.replace("|", "\\|"),
                )
            )
    if late_rows:
        lines.extend([
            "",
            "## Late Placement Candidates",
            "",
            "These rows are non-failing review leads. They use only source/ref-backed evidence; filename-only matches remain path hints.",
            "",
            "| Test | Current | Suggested Earliest Owner | Confidence | Features | Reason | Evidence | Path Hints |",
            "| --- | --- | --- | --- | --- | --- | --- | --- |",
        ])
        for row in late_rows:
            candidate = row["late_placement_candidate"]  # type: ignore[index]
            current = f"{row['current_pa']}:{row['current_cluster']}"
            suggested = f"{candidate['suggested_pa']}:{candidate['suggested_cluster']}"  # type: ignore[index]
            evidence = ", ".join(candidate.get("evidence", [])[:4])  # type: ignore[union-attr]
            lines.append(
                "| `{}` | `{}` | `{}` | `{}` | {} | {} | `{}` | {} |".format(
                    row["path"],
                    current,
                    suggested,
                    candidate.get("confidence", ""),  # type: ignore[union-attr]
                    markdown_cell(candidate.get("features", [])),  # type: ignore[union-attr]
                    markdown_cell(candidate.get("reason", "")),  # type: ignore[union-attr]
                    evidence.replace("|", "\\|"),
                    markdown_cell(row["path_hint_features"]),
                )
            )
    return "\n".join(lines) + "\n"


def markdown_cell(value: object) -> str:
    if isinstance(value, list):
        text = ", ".join(str(item) for item in value)
    elif value is None:
        text = ""
    else:
        text = str(value)
    return text.replace("|", "\\|")


def selected_pas_for_rows(rows: Iterable[dict[str, object]]) -> list[str]:
    pas = {str(row["current_pa"]) for row in rows}
    return sorted(pas, key=lambda pa: pa_number(pa) or 0)


def template_tracker_title(pas: list[str]) -> str:
    if pas == ["pa18"]:
        return "PA18 Template Placement Tracker"
    if tuple(pas) == STRICT_TEMPLATE_PAS:
        return "Strict Template Placement Tracker"
    return "Template Placement Tracker"


def template_tracker_output_path(pas: list[str]) -> str:
    if pas == ["pa18"]:
        return "obj/pa18-template-placement-tracker.md"
    if tuple(pas) == STRICT_TEMPLATE_PAS:
        return "obj/template-strict-placement-tracker.md"
    return "obj/template-placement-tracker.md"


def template_tracker_scope_label(pas: list[str]) -> str:
    if pas == ["pa18"]:
        return "PA18"
    if tuple(pas) == STRICT_TEMPLATE_PAS:
        return "the strict template PAs (`pa14 pa15 pa17 pa18 pa19`)"
    return "the selected template PAs (`{}`)".format(" ".join(pas))


def template_tracker_report(rows: list[dict[str, object]], missing_rules: list[str]) -> str:
    bucket_counts: dict[str, int] = {}
    for row in rows:
        bucket = str(row["template_bucket"])
        bucket_counts[bucket] = bucket_counts.get(bucket, 0) + 1
    pas = selected_pas_for_rows(rows)
    pa_args = " ".join(f"--pa {pa}" for pa in pas)
    output_path = template_tracker_output_path(pas)
    lines = [
        f"# {template_tracker_title(pas)}",
        "",
        f"This tracker is the review queue for template test placement across {template_tracker_scope_label(pas)}.",
        "It supports the split into:",
        "",
        "- PA14/PA15/PA17 basic template owners",
        "- PA18 advanced single-feature template completion",
        "- PA19 template integration",
        "- later owners, split/reduce, or drop decisions",
        "",
        "The table below was seeded by the template-placement audit mode.",
        "Treat the bucket and cluster as review leads, not final move decisions.",
        "The composition-concepts column retains earlier prerequisite concepts that PA18's owner-focused view filters out.",
        "Filename-only matches are retained as path hints and do not drive placement failures.",
        "After review starts, do not overwrite this tracker without preserving status and notes.",
        "",
        "Seed command:",
        "",
        "```sh",
        f"python3 scripts/audit_pa_feature_placement.py {pa_args} --template-placement \\",
        f"  --markdown-out {output_path} \\",
        "  --csv-out /tmp/template-placement.csv \\",
        "  --json-out /tmp/template-placement.json",
        "```",
        "",
        "Status legend: `[ ]` todo · `[~]` in progress · `[x]` placed · `[D]` dropped · `[-]` deferred",
        "",
        "## Review Rules",
        "",
        "- A test goes to the earliest PA/cluster that owns the behavior it asserts.",
        "- Support syntax does not control placement when it is already implemented and not essential to the expected output.",
        "- If two or more template concepts are essential together, place the test in PA19 integration and cluster it by the feature combination.",
        "- If a later non-template feature is essential, move later or split/reduce the test before keeping template coverage.",
        "- Witness refs are golden; do not regenerate witness refs while moving tests.",
        "",
        "## PA19 Candidate Clusters",
        "",
        "| Cluster | Intended integration shape |",
        "| --- | --- |",
        "| 100 | dependent-name/entity interactions that do not fit a narrower later cluster |",
        "| 200 | deduction, partial ordering, non-deduced contexts, and braced-init deduction combinations |",
        "| 300 | SFINAE, substitution, detector idiom, and no-eager instantiation combinations |",
        "| 400 | pack, member-template, template-template-parameter, alias-template, and variable-template compositions |",
        "| 500 | library-shaped end-to-end reducers without hosted/builtin dependencies |",
        "",
        "## Generated Summary",
        "",
        f"- tests scanned: {len(rows)}",
        f"- feature table entries without detector rules: {len(missing_rules)}",
    ]
    for bucket in sorted(bucket_counts):
        lines.append(f"- {bucket}: {bucket_counts[bucket]}")
    lines.extend([
        "",
        "## Review Queue",
        "",
        "| Status | Test | Current | Bucket | Concepts For Review | Composition Concepts | Later/Compat Features | Latest Template Owner | PA19 Cluster | Late Candidate | Late Confidence | Path Hints | Action | Notes |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ])
    for row in sorted(rows, key=lambda item: (str(item["template_bucket"]), str(item["path"]))):
        current = f"{row['current_pa']}:{row['current_cluster']}"
        late_candidate = row.get("late_placement_candidate") or {}
        late_label = (
            placement_label(
                late_candidate.get("suggested_pa", ""),  # type: ignore[union-attr]
                late_candidate.get("suggested_cluster"),  # type: ignore[union-attr]
            )
            if late_candidate else ""
        )
        lines.append(
            "| [ ] | `{}` | `{}` | `{}` | {} | {} | {} | `{}` | {} | `{}` | `{}` | {} | {} |  |".format(
                row["path"],
                current,
                row["template_bucket"],
                markdown_cell(row["review_template_concepts"]),
                markdown_cell(row["integration_template_concepts"]),
                markdown_cell(row["later_or_compat_features"]),
                row["latest_template_owner"],
                markdown_cell(row["suggested_pa18_cluster"]),
                late_label,
                late_candidate.get("confidence", "") if late_candidate else "",  # type: ignore[union-attr]
                markdown_cell(row["path_hint_features"]),
                markdown_cell(row["template_action"]),
            )
        )
    return "\n".join(lines) + "\n"


def write_json(path: Path,
               rows: list[dict[str, object]],
               missing_rules: list[str],
               lowir_eh_findings: list[LowIREHFinding] | None = None) -> None:
    payload = {
        "tests": rows,
        "missing_detector_rules": missing_rules,
        "lowir_eh_review_findings": [
            {
                "path": finding.path,
                "kind": finding.kind,
                "evidence": finding.evidence,
            }
            for finding in (lowir_eh_findings or [])
        ],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_csv(path: Path, rows: list[dict[str, object]], template_placement: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        if template_placement:
            writer.writerow([
                "path",
                "current_pa",
                "current_cluster",
                "test_role",
                "expected_exit",
                "template_bucket",
                "template_concept_arity",
                "review_template_concepts",
                "integration_template_concept_arity",
                "integration_template_concepts",
                "template_features",
                "later_or_compat_features",
                "latest_template_owner",
                "suggested_pa18_cluster",
                "template_action",
                "late_candidate_owner",
                "late_candidate_confidence",
                "late_candidate_features",
                "path_hint_features",
                "source_sidecars_scanned",
            ])
            for row in rows:
                late_candidate = row.get("late_placement_candidate") or {}
                writer.writerow([
                    row["path"],
                    row["current_pa"],
                    row["current_cluster"],
                    row["test_role"],
                    row["expected_exit"],
                    row["template_bucket"],
                    row["template_concept_arity"],
                    "; ".join(row["review_template_concepts"]),  # type: ignore[arg-type]
                    row["integration_template_concept_arity"],
                    "; ".join(row["integration_template_concepts"]),  # type: ignore[arg-type]
                    "; ".join(row["template_features"]),  # type: ignore[arg-type]
                    "; ".join(row["later_or_compat_features"]),  # type: ignore[arg-type]
                    row["latest_template_owner"],
                    row["suggested_pa18_cluster"] or "",
                    row["template_action"],
                    placement_label(
                        late_candidate.get("suggested_pa", ""),  # type: ignore[union-attr]
                        late_candidate.get("suggested_cluster"),  # type: ignore[union-attr]
                    ) if late_candidate else "",
                    late_candidate.get("confidence", "") if late_candidate else "",  # type: ignore[union-attr]
                    "; ".join(late_candidate.get("features", [])) if late_candidate else "",  # type: ignore[union-attr]
                    "; ".join(row["path_hint_features"]),  # type: ignore[arg-type]
                    row["source_sidecars_scanned"],
                ])
            return
        writer.writerow([
            "path",
            "current_pa",
            "current_cluster",
            "test_role",
            "expected_exit",
            "feature",
            "owner_pa",
            "owner_cluster",
            "status",
            "reason",
            "evidence",
            "late_candidate_owner",
            "late_candidate_confidence",
            "late_candidate_features",
            "path_hint_features",
        ])
        for row in rows:
            late_candidate = row.get("late_placement_candidate") or {}
            for placement in row["placements"]:  # type: ignore[index]
                writer.writerow([
                    row["path"],
                    row["current_pa"],
                    row["current_cluster"],
                    row["test_role"],
                    row["expected_exit"],
                    placement["feature"],
                    placement.get("owner_pa", ""),
                    placement.get("owner_cluster", ""),
                    placement["status"],
                    placement["reason"],
                    "; ".join(placement.get("evidence", [])),
                    placement_label(
                        late_candidate.get("suggested_pa", ""),  # type: ignore[union-attr]
                        late_candidate.get("suggested_cluster"),  # type: ignore[union-attr]
                    ) if late_candidate else "",
                    late_candidate.get("confidence", "") if late_candidate else "",  # type: ignore[union-attr]
                    "; ".join(late_candidate.get("features", [])) if late_candidate else "",  # type: ignore[union-attr]
                    "; ".join(row["path_hint_features"]),  # type: ignore[arg-type]
                ])


def early_placement_findings(rows: list[dict[str, object]]) -> list[tuple[dict[str, object], dict[str, object]]]:
    findings: list[tuple[dict[str, object], dict[str, object]]] = []
    for row in rows:
        for placement in row["placements"]:  # type: ignore[index]
            if placement["status"] in EARLY_PLACEMENT_STATUSES:
                findings.append((row, placement))
    return findings


def github_escape_message(text: object) -> str:
    return str(text).replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def github_escape_property(text: object) -> str:
    escaped = github_escape_message(text)
    return escaped.replace(":", "%3A").replace(",", "%2C")


def placement_label(pa: object, cluster: object) -> str:
    if cluster is None:
        return str(pa)
    return f"{pa}:{cluster}"


def emit_early_placement_errors(findings: list[tuple[dict[str, object], dict[str, object]]]) -> None:
    count = len(findings)
    plural = "" if count == 1 else "s"
    print(
        f"error: test placement audit found {count} feature use{plural} placed "
        "before the owning PA/cluster.",
        file=sys.stderr,
    )
    print(
        "Move the test to the owner shown below, or reduce the test so it no "
        "longer depends on the later feature.",
        file=sys.stderr,
    )
    for row, placement in findings:
        current = placement_label(row["current_pa"], row["current_cluster"])
        owner = placement_label(placement.get("owner_pa", ""), placement.get("owner_cluster"))
        evidence = "; ".join(placement.get("evidence", [])[:2])
        message = (
            f"{placement['feature']} belongs in {owner}, but this test is in "
            f"{current}: {placement['reason']}. Evidence: {evidence}"
        )
        path = row["path"]
        print(
            "::error file={},title={}::{}".format(
                github_escape_property(path),
                github_escape_property("Test placed before owning feature"),
                github_escape_message(message),
            ),
            file=sys.stderr,
        )
        print(f"- {path}: {message}", file=sys.stderr)


def emit_hygiene_errors(findings: list[HygieneFinding]) -> None:
    count = len(findings)
    plural = "" if count == 1 else "s"
    print(
        f"error: test placement audit found {count} local test hygiene "
        f"finding{plural}.",
        file=sys.stderr,
    )
    for finding in findings:
        evidence = f" Evidence: {finding.evidence}" if finding.evidence else ""
        message = f"{finding.message}.{evidence}"
        print(
            "::error file={},title={}::{}".format(
                github_escape_property(finding.path),
                github_escape_property("Local test hygiene finding"),
                github_escape_message(message),
            ),
            file=sys.stderr,
        )
        print(f"- {finding.path}: {message}", file=sys.stderr)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--tracker", type=Path, default=DEFAULT_TRACKER)
    parser.add_argument("--pa", action="append", choices=COURSE_PAS)
    parser.add_argument("--feature", action="append", help="only report tests matching this feature id")
    parser.add_argument(
        "--include-ok",
        action="store_true",
        help="include ok and semantic-stage rows in markdown",
    )
    parser.add_argument(
        "--template-placement",
        action="store_true",
        help="emit a per-test template-placement review queue instead of placement findings",
    )
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--csv-out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument(
        "--fail-on-early",
        action="store_true",
        help="exit nonzero when a test uses a feature before its owning PA/cluster",
    )
    parser.add_argument(
        "--lowir-probe-app",
        type=Path,
        help="compiler used only for explicitly selected --probe-lowir-test anchors",
    )
    parser.add_argument(
        "--probe-lowir-test",
        action="append",
        type=Path,
        default=[],
        help=(
            "repo-relative host-test .t anchor to inspect with --emit-lowir; "
            "repeat for ambiguous tests"
        ),
    )
    args = parser.parse_args(argv)

    root = args.root.resolve()
    tracker = args.tracker if args.tracker.is_absolute() else root / args.tracker
    if not tracker.exists():
        print(f"error: tracker not found: {tracker}", file=sys.stderr)
        return 2
    features = load_feature_table(tracker)
    owner_errors = feature_owner_errors(features)
    if owner_errors:
        for error in owner_errors:
            print(f"error: {tracker}: {error}", file=sys.stderr)
        return 2
    rule_ids = {rule.feature_id for rule in RULES}
    missing_rules = sorted(set(features) - rule_ids)

    pas = tuple(args.pa) if args.pa else DEFAULT_PAS
    hygiene_pas = tuple(args.pa) if args.pa else LOCAL_TEST_HYGIENE_PAS
    tests = iter_test_files(root, pas)
    if not args.pa:
        # Every required control participates, including backend and host PAs.
        for pa in COURSE_PAS:
            if pa not in pas:
                tests.extend(suite_test_files(root / pa / "tests" / "controls"))
    rows = [row_for(path, root, features) for path in tests]
    hygiene_findings = scan_test_hygiene(root, hygiene_pas)
    lowir_eh_findings = scan_lowir_eh_review(root, hygiene_pas)
    if args.probe_lowir_test:
        if args.lowir_probe_app is None:
            print(
                "error: --probe-lowir-test requires --lowir-probe-app",
                file=sys.stderr,
            )
            return 2
        compiler = (
            args.lowir_probe_app
            if args.lowir_probe_app.is_absolute()
            else root / args.lowir_probe_app
        ).resolve()
        if not compiler.is_file():
            print(f"error: LowIR probe compiler not found: {compiler}", file=sys.stderr)
            return 2
        probe_tests: list[Path] = []
        for requested in args.probe_lowir_test:
            path = requested if requested.is_absolute() else root / requested
            path = path.resolve()
            try:
                relative = path.relative_to(root)
            except ValueError:
                print(f"error: LowIR probe test is outside the root: {path}", file=sys.stderr)
                return 2
            if not path.is_file() or path.suffix != ".t":
                print(f"error: LowIR probe test is not a .t anchor: {relative}", file=sys.stderr)
                return 2
            probe_tests.append(path)
        generated_lowir_by_test: dict[str, str] = {}
        lowir_eh_findings.extend(scan_generated_lowir_eh_review(
            root,
            compiler,
            probe_tests,
            generated_lowir_by_test,
        ))
        rows_by_path = {str(row["path"]): row for row in rows}
        for relative_path, lowir in generated_lowir_by_test.items():
            row = rows_by_path.get(relative_path)
            if row is None:
                continue
            path = root / relative_path
            source = read_text(path) + "\n" + companion_source_text_for(path)
            for feature_id, hit in generated_lowir_feature_hits(
                source,
                lowir,
                relative_path,
            ).items():
                add_row_feature(
                    row,
                    feature_id,
                    ", ".join(hit.evidence),
                    features,
                )
        for finding in lowir_eh_findings:
            if finding.kind != "generated-eh-control":
                continue
            row = rows_by_path.get(finding.path)
            if row is not None and row["current_pa"] == "pa26":
                add_row_feature(
                    row,
                    "host.eh_object",
                    finding.evidence,
                    features,
                )
    if args.feature:
        wanted = set(args.feature)
        rows = [
            row for row in rows
            if wanted.intersection(set(row["detected_features"]))  # type: ignore[arg-type]
        ]
        retained_paths = {str(row["path"]) for row in rows}
        lowir_eh_findings = [
            finding for finding in lowir_eh_findings
            if finding.path in retained_paths
        ]
    if args.json_out:
        write_json(args.json_out if args.json_out.is_absolute() else root / args.json_out,
                   rows,
                   missing_rules,
                   lowir_eh_findings)
    if args.csv_out:
        write_csv(args.csv_out if args.csv_out.is_absolute() else root / args.csv_out,
                  rows,
                  args.template_placement)
    if args.template_placement:
        report = template_tracker_report(rows, missing_rules)
    else:
        report = markdown_report(
            rows,
            missing_rules,
            args.include_ok,
            hygiene_findings,
            lowir_eh_findings,
        )
    if args.markdown_out:
        out = args.markdown_out if args.markdown_out.is_absolute() else root / args.markdown_out
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(report, encoding="utf-8")
    else:
        print(report, end="")
    if args.fail_on_early:
        early_findings = early_placement_findings(rows)
        probe_failures = [
            finding
            for finding in lowir_eh_findings
            if finding.kind == "lowir-probe-failed"
        ]
        if early_findings or hygiene_findings or probe_failures:
            if hygiene_findings:
                emit_hygiene_errors(hygiene_findings)
            if early_findings:
                emit_early_placement_errors(early_findings)
            for finding in probe_failures:
                print(
                    f"error: LowIR placement probe failed for {finding.path}: "
                    f"{finding.evidence}",
                    file=sys.stderr,
                )
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
