// Translated from sema/template_packs.cpp::pack_expansion_type/unexpanded_packs.
// See NOTICE.
//! Validate the formed declarator pattern, independently of pack substitution.
use super::{
    Error,
    analyzer::Analyzer,
    graph::EntityId,
    templates::HeadId,
    types::{TypeId, TypeKind},
};
use crate::support::hash::HashSet;
use crate::support::source::SourceOffset;

#[derive(Default)]
pub(super) struct PackPattern {
    pub parameters: Vec<EntityId>,
    pub bound: Vec<TypeId>,
    pub expression: bool,
}

impl Analyzer<'_> {
    pub(super) fn check_pack_pattern(
        &self,
        pattern: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let valid = if self.template_depth == 0 {
            false
        } else {
            let packs = self.type_pack_pattern(pattern, true);
            !packs.parameters.is_empty() || !packs.bound.is_empty() || packs.expression
        };
        if !valid {
            return Err(Error::at(
                location,
                "a pack expansion that names no parameter pack",
            ));
        }
        Ok(())
    }

    pub(super) fn check_function_packs(
        &self,
        ty: TypeId,
        head: HeadId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self
            .graph
            .templates
            .parameters(head)
            .iter()
            .any(|&parameter| {
                self.graph
                    .entity(parameter)
                    .template_parameter
                    .is_some_and(super::templates::ParameterInfo::is_pack)
            })
            && self.has_unexpanded_type_pack(ty, false)
        {
            return Err(Error::at(
                location,
                "a function declaration names a parameter pack it does not expand",
            ));
        }
        Ok(())
    }

    /// Packs named by the represented type outside nested expansions. Each
    /// canonical type is visited once, including qualifiers and dependent
    /// specialization arguments; a template's declaration is not an argument.
    /// Function declarations leave expression placeholders out, as C++ does;
    /// expansion formation includes the names in their retained expressions.
    pub(super) fn has_unexpanded_type_pack(&self, root: TypeId, placeholders: bool) -> bool {
        let packs = self.type_pack_pattern(root, placeholders);
        !packs.parameters.is_empty() || packs.expression
    }

    pub(super) fn type_pack_pattern(&self, root: TypeId, placeholders: bool) -> PackPattern {
        let mut packs = PackPattern::default();
        let mut work = vec![root];
        let mut visited = HashSet::default();
        let mut parameters = HashSet::default();
        while let Some(ty) = work.pop() {
            if !visited.insert(ty) {
                continue;
            }
            match &self.graph.types.get(ty).kind {
                TypeKind::TemplateParameter { entity, .. } => {
                    if let Some(name) = self.graph.class_templates.dependent_names.get(*entity) {
                        work.push(name.qualifier);
                    } else if let Some(member) = self.graph.dependent_members.get(*entity) {
                        work.push(member.qualifier);
                    } else if let Some(pattern) = self.graph.template_operands.pattern(*entity) {
                        if placeholders && let Some(ty) = pattern.pack_names {
                            packs.expression = true;
                            work.push(ty);
                        }
                    } else if self
                        .graph
                        .entity(*entity)
                        .template_parameter
                        .is_some_and(super::templates::ParameterInfo::is_pack)
                        && parameters.insert(*entity)
                    {
                        packs.parameters.push(*entity);
                    }
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self.graph.class_templates.instance(*entity)
                        && instance.dependent
                    {
                        if let Some(name) = self
                            .graph
                            .class_templates
                            .dependent_names
                            .get(instance.primary)
                        {
                            work.push(name.qualifier);
                        }
                        if let Some(name) = self
                            .graph
                            .class_templates
                            .template_entities
                            .name(instance.primary)
                        {
                            work.push(name);
                        }
                        work.extend(instance.arguments.iter().rev().copied());
                    }
                }
                TypeKind::Pointer(element)
                | TypeKind::Complex(element)
                | TypeKind::Reference { element, .. } => {
                    work.push(*element);
                }
                TypeKind::Array {
                    element, operand, ..
                }
                | TypeKind::Vector {
                    element, operand, ..
                } => {
                    work.push(*element);
                    work.extend(operand);
                }
                TypeKind::MemberPointer { owner, element } => {
                    work.extend([*element, *owner]);
                }
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters.iter().rev().copied());
                }
                TypeKind::Pack(elements) => work.extend(elements.iter().rev().copied()),
                TypeKind::BoundPack { .. } => packs.bound.push(ty),
                TypeKind::Expansion(_)
                | TypeKind::Auto(_)
                | TypeKind::Scalar(_)
                | TypeKind::Template(_)
                | TypeKind::Enum(_)
                | TypeKind::Value { .. }
                | TypeKind::Address { .. }
                | TypeKind::BlockPointer(_) => (),
            }
        }
        packs
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        lookup::Filter,
        types::{Cv, Scalar},
    };
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn pack_rejection_restores_template_context_without_publishing_function() {
        for (source, message) in [
            (
                "template<class T>void f(T... args);",
                "a pack expansion that names no parameter pack",
            ),
            (
                "template<class... Ts>void f(Ts value);",
                "a function declaration names a parameter pack it does not expand",
            ),
            (
                "template<class T>void f(void&... value);",
                "reference to void",
            ),
        ] {
            let source = SourceBuffer::new(source.as_bytes().to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-publication").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let name = cursor.intern_name(b"f").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let syntax_count = analyzer.tree.declarators.mark();
            let result = analyzer.declarations(declarations, analyzer.graph.global);
            assert!(
                matches!(result, Err(Error::Diagnostic { message: actual, .. }) if actual == message)
            );
            assert_eq!(analyzer.template_depth, 0);
            assert!(analyzer.template_head.is_none());
            assert!(
                analyzer
                    .graph
                    .own(analyzer.graph.global, name, Filter::Ordinary)
                    .is_none()
            );
            assert_eq!(syntax_count, analyzer.tree.declarators.mark());
        }
    }

    #[test]
    fn pack_queries_follow_typed_owners_and_shared_type_dags() {
        let source =
            SourceBuffer::new(b"template<class... Ts>struct A;template<class T>struct B;".to_vec())
                .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let b = cursor.intern_name(b"B").unwrap();
        let member_name = cursor.intern_name(b"type").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let mut params = Vec::new();
        let mut primaries = Vec::new();
        for name in [a, b] {
            let binding = analyzer
                .graph
                .own(analyzer.graph.global, name, Filter::Ordinary)
                .unwrap();
            let primary = analyzer.graph.binding(binding).entity;
            let head = analyzer.graph.entity(primary).template_head.unwrap();
            let parameter = analyzer.graph.templates.parameters(head)[0];
            params.push(analyzer.graph.entity(parameter).ty.unwrap());
            primaries.push(primary);
        }
        let pack = params[0];
        let ordinary = params[1];
        let scalar = analyzer.graph.types.scalar(Scalar::Void).unwrap();
        assert!(analyzer.has_unexpanded_type_pack(pack, false));
        assert!(!analyzer.has_unexpanded_type_pack(ordinary, false));
        // The primary template's own parameter pack is not part of B<T>'s arguments.
        let cls = analyzer
            .class_specialization(primaries[1], vec![pack])
            .unwrap();
        let cls = analyzer.graph.entity(cls).ty.unwrap();
        let member = analyzer
            .dependent_member_binding(pack, member_name)
            .unwrap();
        let member = analyzer.graph.binding(member).ty.unwrap();
        let pointer = analyzer.graph.types.pointer(pack).unwrap();
        let member_pointer = analyzer.graph.types.member_pointer(pack, scalar).unwrap();
        for ty in [cls, member, pointer, member_pointer] {
            assert!(analyzer.has_unexpanded_type_pack(ty, false));
        }
        let mut absent = ordinary;
        let mut present = pack;
        for _ in 0..2048 {
            for ty in [&mut absent, &mut present] {
                *ty = analyzer
                    .graph
                    .types
                    .intern(
                        TypeKind::Function {
                            result: *ty,
                            parameters: vec![*ty, *ty],
                            variadic: false,
                            cv: Cv::default(),
                            reference: None,
                        },
                        Cv::default(),
                    )
                    .unwrap();
            }
        }
        assert!(!analyzer.has_unexpanded_type_pack(absent, false));
        assert!(analyzer.has_unexpanded_type_pack(present, false));
    }
    #[test]
    fn expression_pack_names_use_definition_scope_and_skip_nested_expansions() {
        use crate::syntax::ast::{self, DeclKind, ExprKind, SpecifierKind};
        let source =
            SourceBuffer::new(b"template<class... Ts>struct A;decltype(Ts()) f();".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"expression-pack-scope").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let declarations = analyzer.tree.decl_lists.get(declarations);
        let (first, second) = (declarations[0], declarations[1]);
        analyzer.declaration(first, analyzer.graph.global).unwrap();
        let binding = analyzer
            .graph
            .own(analyzer.graph.global, a, Filter::Ordinary)
            .unwrap();
        let head = analyzer
            .graph
            .entity(analyzer.graph.binding(binding).entity)
            .template_head
            .unwrap();
        let scope = analyzer.graph.templates.parameter_scope(head);
        let DeclKind::Simple { specifiers, .. } = analyzer.tree.declarations.get(second).kind
        else {
            panic!("simple declaration");
        };
        let SpecifierKind::Decltype(expression) = analyzer.tree.specs.get(specifiers)[0].kind
        else {
            panic!("decltype");
        };
        assert!(analyzer.expression_names_pack(expression, scope).unwrap());
        assert!(
            !analyzer
                .expression_names_pack(expression, analyzer.graph.global)
                .unwrap()
        );
        let location = analyzer.tree.expressions.get(expression).location;
        let nested = analyzer
            .tree
            .expressions
            .add(ast::Expression {
                location,
                kind: ExprKind::PackExpansion(expression),
            })
            .unwrap();
        assert!(!analyzer.expression_names_pack(nested, scope).unwrap());
        // Statement blocks use the cloner's lexical walk, and collection must
        // rewind every fact-bearing copy. Nested expansions keep their names.
        for (value, expected) in [(expression, true), (nested, false)] {
            let statement = analyzer
                .tree
                .statements
                .add(ast::Statement {
                    selection_init: false,
                    location,
                    kind: ast::StmtKind::Expression(Some(value)),
                })
                .unwrap();
            let list = analyzer.tree.stmt_lists.begin();
            analyzer.tree.stmt_lists.push(statement);
            let list = analyzer.tree.stmt_lists.finish(list).unwrap();
            let block = analyzer
                .tree
                .statements
                .add(ast::Statement {
                    selection_init: false,
                    location,
                    kind: ast::StmtKind::Compound(list),
                })
                .unwrap();
            let block = analyzer
                .tree
                .expressions
                .add(ast::Expression {
                    location,
                    kind: ExprKind::Statement(block),
                })
                .unwrap();
            let before = (
                analyzer.tree.expressions.mark(),
                analyzer.tree.statements.mark(),
                analyzer.tree.names.mark(),
            );
            assert_eq!(
                analyzer.expression_names_pack(block, scope).unwrap(),
                expected
            );
            assert_eq!(
                (
                    analyzer.tree.expressions.mark(),
                    analyzer.tree.statements.mark(),
                    analyzer.tree.names.mark()
                ),
                before
            );
        }
        analyzer.template_depth = 1;
        let ty = analyzer
            .dependent_operand(expression, scope, false)
            .unwrap();
        // Model later lookup-state changes: the placeholder keeps the fact
        // captured at formation, even after its original name is no longer found.
        analyzer.graph.scope_mut(scope).index.clear();
        assert!(!analyzer.expression_names_pack(expression, scope).unwrap());
        assert!(analyzer.has_unexpanded_type_pack(ty, true));
        assert!(!analyzer.has_unexpanded_type_pack(ty, false));
    }
}
