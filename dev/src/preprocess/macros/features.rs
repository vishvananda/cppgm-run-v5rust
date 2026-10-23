// Translated from preprocess/macro/feature_probes.cpp. See NOTICE.
//! The frozen language guarantees, queried with explicit invocation modes.
use crate::preprocess::options::FeatureModes;
const LANGUAGE_FEATURES: &[&[u8]] = &[
    b"c_atomic",
    b"cxx_access_control_sfinae",
    b"cxx_alias_templates",
    b"cxx_alignas",
    b"cxx_alignof",
    b"cxx_atomic",
    b"cxx_attributes",
    b"cxx_auto_type",
    b"cxx_binary_literals",
    b"cxx_constexpr",
    b"cxx_decltype",
    b"cxx_decltype_incomplete_return_types",
    b"cxx_default_function_template_args",
    b"cxx_defaulted_functions",
    b"cxx_delegating_constructors",
    b"cxx_deleted_functions",
    b"cxx_explicit_conversions",
    b"cxx_generalized_initializers",
    b"cxx_implicit_moves",
    b"cxx_inheriting_constructors",
    b"cxx_inline_namespaces",
    b"cxx_lambdas",
    b"cxx_local_type_template_args",
    b"cxx_noexcept",
    b"cxx_nonstatic_member_init",
    b"cxx_nullptr",
    b"cxx_override_control",
    b"cxx_range_for",
    b"cxx_raw_string_literals",
    b"cxx_reference_qualified_functions",
    b"cxx_rvalue_references",
    b"cxx_static_assert",
    b"cxx_strong_enums",
    b"cxx_thread_local",
    b"cxx_trailing_return",
    b"cxx_unicode_literals",
    b"cxx_unrestricted_unions",
    b"cxx_user_literals",
    b"cxx_variable_templates",
    b"cxx_variadic_templates",
];
pub(super) fn is_provided(mut name: &[u8], modes: FeatureModes) -> bool {
    if name.len() > 4
        && let Some(inner) = name.strip_prefix(b"__").and_then(|s| s.strip_suffix(b"__"))
    {
        name = inner;
    }
    match name {
        b"cxx_exceptions" => return modes.exceptions,
        b"cxx_rtti" => return modes.rtti,
        _ => {}
    }
    if LANGUAGE_FEATURES.contains(&name) {
        return true;
    }
    if name.starts_with(b"is_") || name.starts_with(b"has_") || name == b"underlying_type" {
        return crate::lex::builtins::is_trait(&[b"__", name].concat());
    }
    false
}
