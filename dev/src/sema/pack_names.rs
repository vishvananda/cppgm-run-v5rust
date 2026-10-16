// Translated from template_packs.cpp::DefinitionPacks/expression_packs and
// syntax/ast_clone.cpp's collect_packs view. See NOTICE.
//! Read parsed operands in their definition scope; nested expansions own their names.
use super::{Error, analyzer::Analyzer, graph::ScopeId, lookup::Filter};
use crate::syntax::{
    ast::{self, ExprKind, NameKind, SpecifierKind},
    storage::List,
};
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub(super) enum PackNames {
    Known(bool),
    Unsupported(&'static str),
}
impl PackNames {
    pub(super) fn contains_pack(self) -> Result<bool, Error> {
        match self {
            Self::Known(found) => Ok(found),
            Self::Unsupported(feature) => Err(Error::Missing(feature)),
        }
    }
}

#[derive(Clone, Copy)]
enum Probe {
    Expression(ast::ExprId),
    Type(ast::TypeId),
    Name(ast::Name, bool),
    Specifiers(List<ast::Specifier>),
    Declarator(ast::DeclaratorId),
}
impl Analyzer<'_> {
    pub(super) fn expression_pack_names(
        &self,
        expression: ast::ExprId,
        scope: ScopeId,
    ) -> Result<PackNames, Error> {
        match self.expression_names_pack(expression, scope) {
            Ok(found) => Ok(PackNames::Known(found)),
            Err(Error::Missing(feature)) => Ok(PackNames::Unsupported(feature)),
            Err(error) => Err(error),
        }
    }

    pub(super) fn expression_names_pack(
        &self,
        root: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let mut work = vec![Probe::Expression(root)];
        let mut expressions = HashSet::new();
        let mut types = HashSet::new();
        while let Some(probe) = work.pop() {
            match probe {
                Probe::Expression(id) => {
                    if !expressions.insert(id) {
                        continue;
                    }
                    self.pack_expression_probes(id, &mut work)?;
                }
                Probe::Type(id) => {
                    if !types.insert(id) {
                        continue;
                    }
                    let ty = self.tree.types.get(id);
                    work.push(Probe::Specifiers(ty.specifiers));
                    work.extend(ty.declarator.map(Probe::Declarator));
                }
                Probe::Name(id, leading) => {
                    if self.pack_name_probes(id, leading, scope, &mut work)? {
                        return Ok(true);
                    }
                }
                Probe::Specifiers(specifiers) => {
                    for specifier in self.tree.specs.get(specifiers) {
                        match specifier.kind {
                            SpecifierKind::Named(name) | SpecifierKind::Elaborated { name, .. } => {
                                work.push(Probe::Name(name, true));
                            }
                            SpecifierKind::Decltype(expr)
                            | SpecifierKind::Vector { operand: expr, .. } => {
                                work.push(Probe::Expression(expr));
                            }
                            SpecifierKind::Class(_) | SpecifierKind::Enum(_) => {
                                return Err(Error::Missing("inline type definition pack names"));
                            }
                            SpecifierKind::Keyword(_) => (),
                        }
                    }
                }
                Probe::Declarator(id) => self.pack_declarator_probes(id, &mut work),
            }
        }
        Ok(false)
    }
    fn pack_name_probes(
        &self,
        id: ast::Name,
        leading: bool,
        scope: ScopeId,
        work: &mut Vec<Probe>,
    ) -> Result<bool, Error> {
        let name = self.tree.names.get(id);
        for (index, component) in self.tree.name_lists.get(name.components).iter().enumerate() {
            match component.kind {
                NameKind::Identifier(name)
                    if leading && index == 0 && !self.tree.names.get(id).global =>
                {
                    let found =
                        self.unqualified_declaration_result(scope, name.into(), Filter::Ordinary)?;
                    if !found.ambiguous
                        && found.binding.is_some_and(|binding| {
                            self.graph
                                .entity(self.graph.binding(binding).entity)
                                .template_parameter
                                .is_some_and(super::templates::ParameterInfo::is_pack)
                        })
                    {
                        return Ok(true);
                    }
                }
                NameKind::Decltype(expression) | NameKind::DestructorDecltype(expression) => {
                    work.push(Probe::Expression(expression));
                }
                NameKind::Conversion(ty) => work.push(Probe::Type(ty)),
                _ => (),
            }
            if let Some(arguments) = component.arguments {
                for argument in self.tree.template_arguments.get(arguments) {
                    let expanded = argument.pack
                        || matches!(argument.kind, ast::TemplateArgumentKind::Type(ty)
                        if self.tree.types.get(ty).declarator.is_some_and(|d| self.tree.declarators.get(d).pack));
                    if expanded {
                        continue;
                    }
                    work.push(match argument.kind {
                        ast::TemplateArgumentKind::Type(ty) => Probe::Type(ty),
                        ast::TemplateArgumentKind::Expression(expr) => Probe::Expression(expr),
                    });
                }
            }
        }
        Ok(false)
    }
    fn pack_expression_probes(&self, id: ast::ExprId, work: &mut Vec<Probe>) -> Result<(), Error> {
        match self.tree.expressions.get(id).kind {
            ExprKind::Id(name) => work.push(Probe::Name(name, true)),
            ExprKind::Paren(expr)
            | ExprKind::Unary { operand: expr, .. }
            | ExprKind::Postfix { operand: expr, .. }
            | ExprKind::Delete { operand: expr, .. } => work.push(Probe::Expression(expr)),
            ExprKind::Binary { left, right, .. }
            | ExprKind::Subscript {
                base: left,
                index: right,
            } => work.extend([Probe::Expression(left), Probe::Expression(right)]),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => work.extend([
                Probe::Expression(condition),
                Probe::Expression(then_expr),
                Probe::Expression(else_expr),
            ]),
            ExprKind::Call { callee, arguments } => {
                work.push(Probe::Expression(callee));
                work.extend(
                    self.tree
                        .expr_lists
                        .get(arguments)
                        .iter()
                        .copied()
                        .map(Probe::Expression),
                );
            }
            ExprKind::Member { base, member, .. } => {
                work.extend([Probe::Expression(base), Probe::Name(member, false)]);
            }
            ExprKind::Braced { elements, .. } => work.extend(
                self.tree
                    .expr_lists
                    .get(elements)
                    .iter()
                    .copied()
                    .map(Probe::Expression),
            ),
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                work.push(Probe::Specifiers(specifiers));
                work.extend(
                    self.tree
                        .expr_lists
                        .get(arguments)
                        .iter()
                        .copied()
                        .map(Probe::Expression),
                );
                work.extend(braced.map(Probe::Expression));
            }
            ExprKind::Cast { ty, operand, .. } => {
                work.extend([Probe::Type(ty), Probe::Expression(operand)]);
            }
            ExprKind::Sizeof { ty, operand, .. } | ExprKind::TypeTrait { ty, operand, .. } => {
                work.extend(ty.map(Probe::Type));
                work.extend(operand.map(Probe::Expression));
            }
            ExprKind::New {
                placement,
                ty,
                initializer,
                ..
            } => {
                work.push(Probe::Type(ty));
                if let Some(placement) = placement {
                    work.extend(
                        self.tree
                            .expr_lists
                            .get(placement)
                            .iter()
                            .copied()
                            .map(Probe::Expression),
                    );
                }
                if let Some(initializer) = initializer {
                    self.pack_initializer_probes(initializer, work);
                }
            }
            ExprKind::Throw(expression) => work.extend(expression.map(Probe::Expression)),
            ExprKind::Literal(_)
            | ExprKind::Keyword(_)
            | ExprKind::SizeofPack(_)
            | ExprKind::PackExpansion(_) => (),
            ExprKind::Lambda(_) => return Err(Error::Missing("lambda expression pack names")),
        }
        Ok(())
    }
    fn pack_declarator_probes(&self, id: ast::DeclaratorId, work: &mut Vec<Probe>) {
        let declarator = self.tree.declarators.get(id);
        work.extend(declarator.nested.map(Probe::Declarator));
        work.extend(declarator.name.map(|name| Probe::Name(name, true)));
        for attribute in self.tree.attributes.get(declarator.attributes) {
            work.extend(attribute.ty.map(Probe::Type));
            work.extend(attribute.expression.map(Probe::Expression));
        }
        for pointer in self.tree.pointers.get(declarator.pointers) {
            work.extend(pointer.class_name.map(|name| Probe::Name(name, true)));
        }
        for suffix in self.tree.suffixes.get(declarator.suffixes) {
            match *suffix {
                ast::Suffix::Array(bound) => work.extend(bound.map(Probe::Expression)),
                ast::Suffix::Function {
                    parameters,
                    qualifiers,
                } => {
                    for parameter in self.tree.parameters.get(parameters.parameters) {
                        work.push(Probe::Specifiers(parameter.specifiers));
                        work.extend(parameter.declarator.map(Probe::Declarator));
                        if let Some(initializer) = parameter.default {
                            self.pack_initializer_probes(initializer, work);
                        }
                    }
                    for qualifier in self.tree.qualifiers.get(qualifiers) {
                        match qualifier.kind {
                            ast::QualifierKind::Noexcept(expr) => {
                                work.extend(expr.map(Probe::Expression));
                            }
                            ast::QualifierKind::Trailing(ty) => work.push(Probe::Type(ty)),
                            ast::QualifierKind::Throw(types) => work.extend(
                                self.tree
                                    .type_lists
                                    .get(types)
                                    .iter()
                                    .copied()
                                    .map(Probe::Type),
                            ),
                            _ => (),
                        }
                    }
                }
            }
        }
    }
    fn pack_initializer_probes(&self, initializer: ast::Initializer, work: &mut Vec<Probe>) {
        match initializer.kind {
            ast::InitializerKind::Default | ast::InitializerKind::Delete => (),
            ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr) => {
                work.push(Probe::Expression(expr));
            }
            ast::InitializerKind::Parenthesized(arguments) => work.extend(
                self.tree
                    .expr_lists
                    .get(arguments)
                    .iter()
                    .copied()
                    .map(Probe::Expression),
            ),
        }
    }
}
