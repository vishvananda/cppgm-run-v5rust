// Translated from sema/analyzer_declarations.cpp::symbol_attributes. See NOTICE.
//! Entity-owned object attributes survive redeclarations; lowering reads facts only.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{BindingKind, EntityId, Graph, ScopeKind},
    specifiers::{StaticSpecifier, ThreadStorage},
};
use crate::syntax::{ast, storage::List};
use std::collections::HashMap;

#[derive(Default)]
struct Attributes {
    weak: bool,
    section: Option<Box<[u8]>>,
}

#[derive(Default)]
pub(super) struct SymbolAttributes {
    values: HashMap<EntityId, Attributes>,
}

impl Analyzer<'_> {
    pub(super) fn declarator_symbol_attributes(
        &mut self,
        declarator: ast::DeclaratorId,
        leading: List<ast::Attribute>,
    ) -> Result<(), Error> {
        let Some(binding) = self
            .graph
            .declared_bindings
            .get(declarator.index())
            .copied()
            .flatten()
        else {
            return Ok(());
        };
        let binding = self.graph.binding(binding);
        // C++'s explicit variable-template path leaves init.declared null.
        // Rust retains that binding for other facts, but attributes still skip it.
        if self
            .graph
            .variable_instances
            .templates
            .explicit(binding.entity)
        {
            return Ok(());
        }
        self.apply_symbol_attributes(binding.entity, binding.kind, leading)?;
        let mut part = Some(declarator);
        while let Some(id) = part {
            let node = *self.tree.declarators.get(id);
            self.apply_symbol_attributes(binding.entity, binding.kind, node.attributes)?;
            part = node.nested;
        }
        Ok(())
    }

    fn apply_symbol_attributes(
        &mut self,
        entity: EntityId,
        kind: BindingKind,
        list: List<ast::Attribute>,
    ) -> Result<(), Error> {
        for index in 0..self.tree.attributes.get(list).len() {
            let attribute = self.tree.attributes.get(list)[index];
            let weak = attribute.kind == ast::AttributeKind::Weak;
            if (!weak && attribute.kind != ast::AttributeKind::Section)
                || attribute.form == ast::AttributeForm::Standard
                    && attribute.site == ast::AttributeSite::Type
            {
                continue;
            }
            let variable = kind == BindingKind::Variable;
            if !variable && kind != BindingKind::Function {
                if !weak {
                    return Err(Error::at(
                        attribute.location,
                        "a section attribute on a declaration of no object",
                    ));
                }
                continue;
            }
            if !variable && !weak {
                continue;
            }
            let owner = self.graph.entity(entity);
            let home = self
                .graph
                .scope(
                    owner
                        .home
                        .ok_or(Error::Invariant("attributed entity without home"))?,
                )
                .kind;
            let (internal, static_storage, thread_local) = if variable {
                let object = owner
                    .variable
                    .ok_or(Error::Invariant("attributed variable without facts"))?;
                let thread_local = object.thread_storage == ThreadStorage::ThreadLocal;
                (
                    object.internal_linkage,
                    home == ScopeKind::Namespace
                        || object.static_specifier == StaticSpecifier::Present
                        || object.is_extern
                        || thread_local,
                    thread_local,
                )
            } else {
                (
                    owner
                        .function
                        .ok_or(Error::Invariant("attributed function without facts"))?
                        .internal_linkage,
                    true,
                    false,
                )
            };
            if weak {
                if internal
                    || !static_storage
                    || variable && !matches!(home, ScopeKind::Namespace | ScopeKind::Class)
                {
                    return Err(Error::at(
                        attribute.location,
                        "a weak declaration of an entity without external linkage",
                    ));
                }
                self.graph
                    .symbol_attributes
                    .values
                    .entry(entity)
                    .or_default()
                    .weak = true;
                continue;
            }
            if !static_storage {
                return Err(Error::at(
                    attribute.location,
                    "a section attribute on a variable without static storage duration",
                ));
            }
            if thread_local {
                return Err(Error::at(
                    attribute.location,
                    "a section attribute on a thread-local variable is not supported",
                ));
            }
            let expression = attribute
                .expression
                .ok_or(Error::Invariant("section without operand"))?;
            let ast::ExprKind::Literal(literal) = self.tree.expressions.get(expression).kind else {
                return Err(Error::Invariant("section without literal"));
            };
            let name =
                token_safe_section(self.literal_bytes.get(literal.spelling)).ok_or_else(|| {
                    Error::at(
                        attribute.location,
                        "a section name outside ASCII letters, digits, `_` and `.`",
                    )
                })?;
            let own = self
                .graph
                .symbol_attributes
                .values
                .entry(entity)
                .or_default();
            if own
                .section
                .as_deref()
                .is_some_and(|previous| previous != name)
            {
                return Err(Error::at(
                    attribute.location,
                    "a section attribute conflicting with an earlier declaration's",
                ));
            }
            if own.section.is_none() {
                own.section = Some(name.into_boxed_slice());
            }
        }
        Ok(())
    }
}

fn token_safe_section(spelling: &[u8]) -> Option<Vec<u8>> {
    let mut inside = false;
    let mut name = Vec::new();
    for &byte in spelling {
        if byte == b'"' {
            inside = !inside;
        } else if inside && (byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')) {
            name.push(byte);
        } else if inside || !matches!(byte, b' ' | b'\t' | b'\n') {
            return None;
        }
    }
    (!name.is_empty()).then_some(name)
}

impl Graph {
    pub(crate) fn lower_symbol_weak(&self, entity: EntityId) -> bool {
        self.symbol_attributes
            .values
            .get(&entity)
            .is_some_and(|own| own.weak)
    }
    pub(crate) fn lower_variable_section(&self, entity: EntityId) -> Option<&[u8]> {
        if self.symbol_attributes.values.is_empty() {
            return None;
        }
        let mut current = Some(entity);
        while let Some(entity) = current {
            if let Some(section) = self
                .symbol_attributes
                .values
                .get(&entity)
                .and_then(|own| own.section.as_deref())
            {
                return Some(section);
            }
            current = self
                .variable_instances
                .member_source(entity)
                .or_else(|| self.variable_instances.templates.primary(entity))
                .filter(|&source| source != entity);
        }
        None
    }
}
