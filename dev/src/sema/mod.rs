//! Typed declaration analysis on the single PA5 syntax tree. See NOTICE.
mod abi_tags;
mod work;
pub(crate) use work::Work;
mod observations;
pub(crate) use observations::Statistics;
mod address_arguments;
mod aggregate_initialization;
mod alias_templates;
mod aligned_aliases;
mod allocation;
mod analyzer;
mod anonymous_types;
mod array_allocation;
mod arrow_calls;
mod asm_templates;
mod assembly;
mod assignment;
mod atomic_builtins;
mod atomic_operations;
mod base_facts;
mod base_layout;
mod base_projections;
mod bindings;
mod bit_fields;
mod bit_layout;
mod builtin_calls;
mod builtin_operators;
mod call_arguments;
mod call_lookup;
mod call_tables;
mod callable_objects;
mod captures;
mod class_casts;
mod class_conditionals;
mod class_templates;
mod complex_constants;
mod complex_values;
mod const_default;
mod constant_addresses;
mod constant_floats;
mod constant_mutations;
mod constant_pointer_operations;
mod constant_views;
mod constants;
mod constexpr_calls;
mod constexpr_construction;
mod constexpr_frames;
mod constexpr_objects;
mod constexpr_statements;
mod constexpr_values;
mod construction;
mod constructor_initialization;
mod constructor_queries;
mod conversion_functions;
mod conversion_templates;
mod copy_construction;
mod copy_initialization;
mod copy_properties;
mod data_member_pointers;
mod data_members;
mod declaration_access;
mod declarations;
mod declarators;
mod default_arguments;
mod default_construction;
mod default_constructor_facts;
mod deferred_constants;
mod dependence;
mod dependent_members;
mod dependent_templates;
mod designated_initialization;
mod destruction;
mod destructor_expressions;
mod dump;
mod dynamic_casts;
mod empty_layout;
mod enum_instances;
mod enumerations;
mod evaluated_data;
mod exception_operands;
mod exceptions;
mod explicit_classes;
mod explicit_conditions;
mod explicit_instantiations;
mod explicit_specializations;
mod expression_facts;
mod expression_keys;
mod expression_predicates;
mod expressions;
mod final_overriders;
mod fixed_expressions;
mod floating_intrinsics;
mod friendships;
mod function_demands;
mod function_names;
mod function_packs;
mod function_resolution;
mod function_templates;
mod graph;
mod import_access;
mod inheritance;
mod inherited_constructors;
mod initialization;
mod initializer_lists;
mod instance_definitions;
mod integer_intrinsics;
mod integral;
mod interpreter_memo;
mod interpreter_queries;
mod invoke;
mod jumps;
mod lambdas;
mod layout;
mod layout_attributes;
mod library_builtins;
mod linkage;
mod list_arguments;
mod literal_calls;
mod literal_numbers;
mod literal_types;
mod local_classes;
mod local_constants;
mod local_names;
mod locality;
mod lookup;
pub(crate) mod lowering;
mod member_construction;
mod member_expressions;
mod member_initializers;
mod member_patterns;
mod name_keys;
mod nominal_predicates;
mod offset_of;
mod operator_candidates;
mod operator_resolution;
mod overflow_intrinsics;
mod pack_arguments;
mod pack_deduction;
mod pack_formation;
mod pack_frames;
mod pack_names;
mod pack_substitution;
mod placeholders;
mod predefined_names;
mod predefined_variables;
mod predicates;
mod procedural_calls;
mod procedural_statements;
mod property_predicates;
mod qualifier_prefixes;
mod queries;
mod range_for;
mod reserved_wrappers;
mod return_deduction;
mod runtime_intrinsics;
mod scalar_casts;
mod scalar_conversions;
mod scalar_expressions;
mod scalar_operators;
mod semantics_dump;
mod soft_values;
mod source_exceptions;
mod source_locations;
mod special_members;
mod specialized_classes;
mod specifiers;
mod stack_intrinsics;
mod statement_expressions;
mod statements;
mod static_addresses;
mod static_arrays;
mod static_construction;
mod static_initialization;
mod static_objects;
mod substitution_state;
mod symbol_attributes;
mod symbolic_members;
mod template_bodies;
mod template_context;
mod template_defaults;
mod template_exceptions;
mod triviality;
mod type_queries;
mod type_transforms;
mod vector_builtins;
mod vector_constant_bytes;
mod vector_constants;
mod vector_operations;
mod vector_types;
mod wide_integers;
pub(crate) use analyzer::AnalysisMode;
pub(crate) use template_bodies::AnalysisOptions;
#[cfg(test)]
pub(crate) use template_bodies::analyze_unit;
pub(crate) use template_bodies::analyze_unit_with_mode;
mod named_templates;
mod override_search;
mod partial_classes;
mod partial_specializations;
mod template_names;
mod template_operands;
mod templates;
mod throw_queries;
mod types;
mod user_conversions;
mod value_arguments;
mod variable_instances;
mod variable_specializations;
mod variable_templates;
mod virtual_exceptions;
mod virtual_groups;
mod virtual_layout;
mod virtual_members;
mod vtable_groups;
use crate::support::source::SourceOffset;
#[derive(Clone, Copy, Debug)]
pub(crate) enum ConstantFailure {
    Reported,
    Unreported,
}
#[derive(Clone, Debug)]
pub(crate) enum Error {
    Diagnostic {
        location: SourceOffset,
        message: &'static str,
    },
    NonConstant {
        failure: ConstantFailure,
        location: SourceOffset,
        message: &'static str,
    },
    // C++ Analyzer::error throws even while substitution_error is soft.
    UnconditionalDiagnostic {
        location: SourceOffset,
        message: &'static str,
    },
    InstantiationDiagnostic {
        location: SourceOffset,
        message: &'static str,
    },
    /// Already emitted into its owning substitution context; transport only.
    SubstitutionRejected(substitution_state::Failure),
    SubstitutionNull,
    DeferredBuiltin,
    Missing(&'static str),
    Limit(&'static str),
    Invariant(&'static str),
}
impl Error {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Result::map_err transfers the owned interning failure to this phase adapter"
    )]
    pub(super) fn identifier_interning(error: crate::syntax::ParseError) -> Self {
        match error {
            crate::syntax::ParseError::Preprocess(
                crate::preprocess::error::PreprocessError::Limit(message),
            )
            | crate::syntax::ParseError::Limit(message) => Self::Limit(message),
            _ => Self::Invariant("unexpected identifier interning error"),
        }
    }
    pub(super) fn required_expression(self, location: SourceOffset) -> Self {
        match self {
            Self::NonConstant {
                failure: ConstantFailure::Unreported,
                ..
            } => Self::hard(location, "not an integral constant expression"),
            other => other.required_constant(),
        }
    }
    pub(super) fn required_constant(self) -> Self {
        match self {
            Self::NonConstant {
                location, message, ..
            } => Self::hard(location, message),
            other => other,
        }
    }
    pub(super) fn constant_initializer(self, location: SourceOffset) -> Self {
        match self {
            Self::NonConstant {
                failure: ConstantFailure::Unreported,
                ..
            } => Self::hard(location, "a constexpr variable is not constant-initialized"),
            other => other.required_constant(),
        }
    }

    pub(super) fn required_instantiation(self) -> Self {
        match self {
            Self::Diagnostic { location, message }
            | Self::UnconditionalDiagnostic { location, message }
            | Self::NonConstant {
                location, message, ..
            } => Self::InstantiationDiagnostic { location, message },
            other => other,
        }
    }
    pub(super) fn hard(location: SourceOffset, message: &'static str) -> Self {
        Self::UnconditionalDiagnostic { location, message }
    }
    pub(super) fn at(location: SourceOffset, message: &'static str) -> Self {
        Self::Diagnostic { location, message }
    }
}
pub(crate) use analyzer::Analyzer;
pub(crate) use graph::Graph;

fn record<T: Copy>(facts: &mut Vec<Option<T>>, index: usize, fact: T) {
    if index >= facts.len() {
        facts.resize(index + 1, None);
    }
    facts[index] = Some(fact);
}

#[cfg(test)]
mod tests;
