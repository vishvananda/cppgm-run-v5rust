//! Typed declaration analysis on the single PA5 syntax tree. See NOTICE.
mod aggregate_initialization;
mod allocation;
mod analyzer;
mod anonymous_types;
mod array_allocation;
mod arrow_calls;
mod assignment;
mod bit_fields;
mod bit_layout;
mod builtin_calls;
mod builtin_operators;
mod call_arguments;
mod call_lookup;
mod callable_objects;
mod class_casts;
mod class_conditionals;
mod class_templates;
mod constant_addresses;
mod constant_mutations;
mod constant_pointer_operations;
mod constants;
mod construction;
mod constructor_initialization;
mod conversion_functions;
mod copy_construction;
mod copy_initialization;
mod copy_properties;
mod data_members;
mod declaration_access;
mod declarations;
mod declarators;
mod default_arguments;
mod dependence;
mod dependent_members;
mod dependent_templates;
mod destruction;
mod destructor_expressions;
mod dump;
mod empty_layout;
mod enum_instances;
mod enumerations;
mod exception_operands;
mod exceptions;
mod explicit_classes;
mod explicit_instantiations;
mod explicit_specializations;
mod expression_facts;
mod expression_keys;
mod expressions;
mod fixed_expressions;
mod floating_intrinsics;
mod friendships;
mod function_demands;
mod function_names;
mod function_resolution;
mod function_templates;
mod graph;
mod inheritance;
mod inherited_constructors;
mod initialization;
mod instance_definitions;
mod integral;
mod jumps;
mod layout;
mod layout_attributes;
mod linkage;
mod list_arguments;
mod literal_calls;
mod local_classes;
mod local_names;
mod lookup;
pub(crate) mod lowering;
mod member_construction;
mod member_expressions;
mod member_initializers;
mod member_patterns;
mod name_keys;
mod operator_candidates;
mod operator_resolution;
mod pack_formation;
mod pack_names;
mod placeholders;
mod procedural_calls;
mod procedural_statements;
mod queries;
mod scalar_casts;
mod scalar_conversions;
mod scalar_expressions;
mod scalar_operators;
mod semantics_dump;
mod special_members;
mod specifiers;
mod statements;
mod static_arrays;
mod static_construction;
mod static_initialization;
mod static_objects;
mod template_bodies;
mod template_context;
mod template_defaults;
mod template_exceptions;
pub(crate) use template_bodies::analyze_unit;
mod partial_specializations;
mod template_names;
mod template_operands;
mod templates;
mod throw_queries;
mod types;
mod user_conversions;
mod variable_instances;
mod variable_templates;
mod virtual_exceptions;
mod virtual_members;
use crate::support::source::SourceOffset;
#[derive(Debug)]
pub(crate) enum Error {
    Diagnostic {
        location: SourceOffset,
        message: &'static str,
    },
    NonConstant {
        location: SourceOffset,
        message: &'static str,
    },
    InstantiationDiagnostic {
        location: SourceOffset,
        message: &'static str,
    },
    Missing(&'static str),
    Limit(&'static str),
    Invariant(&'static str),
}
impl Error {
    pub(super) fn required_instantiation(self) -> Self {
        match self {
            Self::Diagnostic { location, message } | Self::NonConstant { location, message } => {
                Self::InstantiationDiagnostic { location, message }
            }
            other => other,
        }
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
