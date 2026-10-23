// Translated from sema/template_dependence.cpp and template_placeholders.cpp::name_words.
// See NOTICE.
//! Conservative definition-time type dependence, independent of concrete facts.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::{ExpressionKey, KeyWord},
    graph::{BindingKind, ScopeId, ScopeKind},
    lookup::Filter,
    types::{TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{
        ast::{self, ExprKind, NameKind},
        storage::List,
    },
};

impl Analyzer<'_> {
    pub(super) fn note_dependent_use(&self) {
        self.dependent_uses
            .set(self.dependent_uses.get().wrapping_add(1));
    }

    /// C++ `lookup.cpp::DependentClassScope`, including concrete instances and
    /// local classes: dependence here describes the pattern, not today's type.
    pub(super) fn dependent_lookup_class(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.graph.scope(scope);
            if node.kind != ScopeKind::Class {
                return node.kind != ScopeKind::Namespace;
            }
            if node.owner.is_none_or(|class| {
                self.graph.entity(class).template_head.is_some()
                    || self.graph.class_templates.instance(class).is_some()
                    || self
                        .graph
                        .class_templates
                        .definitions
                        .source(class)
                        .is_some()
            }) {
                return true;
            }
            let Some(parent) = node.parent else {
                return false;
            };
            scope = parent;
        }
    }

    pub(super) fn parameter_dependence(
        &self,
        function: super::graph::EntityId,
        count: usize,
    ) -> Result<Option<TypeId>, Error> {
        let mut source = function;
        for _ in 0..64 {
            let next = self
                .graph
                .function_templates
                .primary(source)
                .map(|binding| self.graph.binding(binding).entity)
                .or_else(|| self.graph.function_templates.bodies.source(source));
            let Some(next) = next.filter(|&next| next != source) else {
                break;
            };
            source = next;
        }
        if source == function {
            return Ok(None);
        }
        let ty = self
            .graph
            .entity(source)
            .ty
            .ok_or(Error::Invariant("parameter pattern without type"))?;
        Ok((self.graph.types.function_parameter_count(ty)? == count).then_some(ty))
    }

    pub(super) fn key_call_resolution(
        &mut self,
        callee: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
        start: usize,
        key: &mut ExpressionKey,
    ) -> Result<(), Error> {
        let ExprKind::Id(name) = self.tree.expressions.get(callee).kind else {
            return Ok(());
        };
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if node.global || components.len() != 1 {
            return Ok(());
        }
        let component = components[0];
        let NameKind::Identifier(name) = component.kind else {
            return Ok(());
        };
        let found = self.unqualified_result(scope, name.into(), Filter::Ordinary)?;
        let Some(binding) = found.binding else {
            return Ok(());
        };
        if self.graph.binding(binding).kind != BindingKind::Function {
            return Ok(());
        }
        let mut dependent = false;
        for index in 0..self.tree.expr_lists.get(arguments).len() {
            let argument = self.tree.expr_lists.get(arguments)[index];
            if self.expression_type_dependence(argument, scope, key.precise)? {
                dependent = true;
                break;
            }
        }
        if !matches!(key.words.get(start + 4), Some(KeyWord::Entity(_))) {
            return Ok(());
        }
        let entity = self.graph.binding(binding).entity;
        let sole = self.semantic() && found.functions.as_slice().len() == 1;
        let class_member = self
            .graph
            .entity(entity)
            .home
            .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class);
        let through_this = class_member
            && self.body_context.is_some()
            && self
                .graph
                .entity(entity)
                .function
                .is_some_and(|function| !function.is_static);
        if dependent {
            key.dependent = true;
        }
        let resolved = if sole
            && !through_this
            && component.arguments.is_none()
            && self.graph.entity(entity).template_head.is_none()
            && (!dependent || class_member)
        {
            Some(entity)
        } else if sole && !through_this && !dependent && component.arguments.is_some() {
            let mut given = Vec::new();
            // These are the canonical arguments already retained by key_name.
            // Resolve only a complete explicit head, never deduce call arguments.
            for word in key.words.get(start + 7..).unwrap_or_default() {
                let KeyWord::Type(ty) = word else {
                    return Err(Error::Invariant("callee key without argument type"));
                };
                given.push(*ty);
            }
            self.key_explicit_specialization(binding, &given, component.location)?
        } else {
            None
        };
        if let Some(resolved) = resolved {
            key.words[start + 4] = KeyWord::ResolvedFunction(resolved);
        } else if dependent {
            key.words[start + 4] = KeyWord::DependentCallee(entity, name);
        }
        Ok(())
    }

    pub(super) fn key_explicit_specialization(
        &mut self,
        binding: super::graph::BindingId,
        given: &[TypeId],
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<super::graph::EntityId>, Error> {
        if given.iter().any(|&ty| self.exception_dependent_type(ty)) {
            return Ok(None);
        }
        let Some(head) = self.function_template_head(binding)? else {
            return Ok(None);
        };
        let Some(converted) = self.match_explicit_template_arguments(head, given, location)? else {
            return Ok(None);
        };
        if converted.len() != self.graph.templates.parameters(head).len() {
            return Ok(None);
        }
        let mut arguments = converted.into_iter().map(Some).collect::<Vec<_>>();
        self.complete_signature(binding, &mut arguments)
            .map(|binding| binding.map(|binding| self.graph.binding(binding).entity))
    }

    pub(super) fn type_dependent_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        self.expression_type_dependence(id, scope, false)
    }

    pub(super) fn expression_type_dependence(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        precise: bool,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        match self.tree.expressions.get(id).kind {
            ExprKind::Literal(_)
            | ExprKind::SizeofPack(_)
            | ExprKind::Sizeof { .. }
            | ExprKind::TypeTrait { .. }
            | ExprKind::Delete { .. }
            | ExprKind::Throw(_) => Ok(false),
            ExprKind::Keyword(keyword) => {
                Ok(keyword == Keyword::This && self.dependent_class_scope(scope))
            }
            ExprKind::Paren(inner)
            | ExprKind::Unary { operand: inner, .. }
            | ExprKind::Postfix { operand: inner, .. } => {
                self.expression_type_dependence(inner, scope, precise)
            }
            ExprKind::Binary { left, right, .. }
            | ExprKind::Subscript {
                base: left,
                index: right,
            } => Ok(self.expression_type_dependence(left, scope, precise)?
                || self.expression_type_dependence(right, scope, precise)?),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => Ok(self.expression_type_dependence(condition, scope, precise)?
                || self.expression_type_dependence(then_expr, scope, precise)?
                || self.expression_type_dependence(else_expr, scope, precise)?),
            ExprKind::Call { callee, arguments } => {
                if self.expression_type_dependence(callee, scope, precise)? {
                    return Ok(true);
                }
                for index in 0..self.tree.expr_lists.get(arguments).len() {
                    let argument = self.tree.expr_lists.get(arguments)[index];
                    if self.expression_type_dependence(argument, scope, precise)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::Member { base, member, .. } => {
                if self.expression_type_dependence(base, scope, precise)? {
                    return Ok(true);
                }
                let name = self.tree.names.get(member);
                Ok(self
                    .tree
                    .name_lists
                    .get(name.components)
                    .iter()
                    .any(|component| {
                        !matches!(component.kind, NameKind::Identifier(_))
                            || component.arguments.is_some()
                    }))
            }
            ExprKind::TypeTransform { types, .. } => {
                for index in 0..self.tree.type_lists.get(types).len() {
                    let ty = self.type_syntax(self.tree.type_lists.get(types)[index], scope)?;
                    if self.exception_dependent_type(ty) {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::VaArg { ty, .. } => {
                let ty = self.type_syntax(ty, scope)?;
                Ok(self.exception_dependent_type(ty))
            }
            ExprKind::Cast { ty, operand, .. } => {
                let ty = self.type_syntax(ty, scope)?;
                Ok(self.exception_dependent_type(ty)
                    || self.expression_type_dependence(operand, scope, precise)?)
            }
            ExprKind::Braced { elements, .. } => {
                for index in 0..self.tree.expr_lists.get(elements).len() {
                    let element = self.tree.expr_lists.get(elements)[index];
                    if self.expression_type_dependence(element, scope, precise)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::Id(name) => self.dependent_name_expression(name, scope, precise),
            // These forms likewise remain conservative in C++'s ordinary
            // dependence walk. Precise member-template checking is separate.
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } if precise => {
                let ty = self.specifiers(specifiers, scope, false)?.ty;
                if self.exception_dependent_type(ty) || braced.is_some() {
                    return Ok(true);
                }
                for index in 0..self.tree.expr_lists.get(arguments).len() {
                    let argument = self.tree.expr_lists.get(arguments)[index];
                    if self.expression_type_dependence(argument, scope, precise)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::Statement(_)
            | ExprKind::FunctionalCast { .. }
            | ExprKind::New { .. }
            | ExprKind::Lambda(_)
            | ExprKind::PackExpansion(_) => Ok(true),
        }
    }

    pub(super) fn dependent_class_scope(&self, mut scope: ScopeId) -> bool {
        loop {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::Class
                && node
                    .owner
                    .is_some_and(|entity| self.graph.entity(entity).template_head.is_some())
            {
                return true;
            }
            let Some(parent) = node.parent else {
                return false;
            };
            scope = parent;
        }
    }

    /// An instantiated class declares its member templates even while their
    /// own parameters remain open. Check only calls/accesses proven fixed;
    /// their enclosing decltype still retains its original dependent operand.
    pub(super) fn check_fixed_decltype_operands(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        level: u32,
    ) -> Result<(), Error> {
        // This is the frozen semantic walk's bound, not a native stack cutoff.
        if level > 64 {
            return Ok(());
        }
        let next = level + 1;
        let kind = self.tree.expressions.get(id).kind;
        if matches!(kind, ExprKind::Call { .. } | ExprKind::Member { .. })
            && !self.expression_type_dependence(id, scope, true)?
            && self
                .fixed_operand_key(id, scope)?
                .is_some_and(|key| !key.dependent)
        {
            let saved = std::mem::replace(&mut self.template_depth, 0);
            let result = self.decltype_type(id, scope);
            self.template_depth = saved;
            return result.map(|_| ());
        }
        match kind {
            ExprKind::Member { base: operand, .. }
            | ExprKind::Paren(operand)
            | ExprKind::Unary { operand, .. }
            | ExprKind::Postfix { operand, .. }
            | ExprKind::Cast { operand, .. } => {
                self.check_fixed_decltype_operands(operand, scope, next)
            }
            ExprKind::Call {
                mut callee,
                arguments,
            } => {
                while let ExprKind::Paren(inner) = self.tree.expressions.get(callee).kind {
                    callee = inner;
                }
                // A member function name is typed together with its call.
                // If that call is dependent, visit only the object and args.
                if let ExprKind::Member { base, .. } = self.tree.expressions.get(callee).kind {
                    self.check_fixed_decltype_operands(base, scope, next)?;
                }
                self.check_fixed_decltype_arguments(arguments, scope, next)
            }
            ExprKind::Binary { left, right, .. } => {
                self.check_fixed_decltype_operands(left, scope, next)?;
                self.check_fixed_decltype_operands(right, scope, next)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.check_fixed_decltype_operands(condition, scope, next)?;
                self.check_fixed_decltype_operands(then_expr, scope, next)?;
                self.check_fixed_decltype_operands(else_expr, scope, next)
            }
            ExprKind::FunctionalCast { arguments, .. } => {
                self.check_fixed_decltype_arguments(arguments, scope, next)
            }
            _ => Ok(()),
        }
    }

    fn check_fixed_decltype_arguments(
        &mut self,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
        level: u32,
    ) -> Result<(), Error> {
        for index in 0..self.tree.expr_lists.get(arguments).len() {
            let argument = self.tree.expr_lists.get(arguments)[index];
            self.check_fixed_decltype_operands(argument, scope, level)?;
        }
        Ok(())
    }

    fn fixed_template_type_arguments(
        &mut self,
        arguments: List<ast::TemplateArgument>,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        for index in 0..self.tree.template_arguments.get(arguments).len() {
            let argument = self.tree.template_arguments.get(arguments)[index];
            let ast::TemplateArgumentKind::Type(ty) = argument.kind else {
                return Ok(false);
            };
            if argument.pack {
                return Ok(false);
            }
            let ty = self.type_syntax(ty, scope)?;
            if self.exception_dependent_type(ty) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn dependent_name_expression(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
        precise: bool,
    ) -> Result<bool, Error> {
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let mut fixed_template_id = false;
        for index in 0..components.len() {
            let component = self.tree.name_lists.get(node.components)[index];
            if !matches!(component.kind, NameKind::Identifier(_)) {
                return Ok(true);
            }
            if let Some(arguments) = component.arguments {
                if !precise
                    || index + 1 != self.tree.name_lists.get(node.components).len()
                    || !self.fixed_template_type_arguments(arguments, scope)?
                {
                    return Ok(true);
                }
                fixed_template_id = true;
            }
        }
        let components = self.tree.name_lists.get(node.components);
        let NameKind::Identifier(name_key) = components
            .last()
            .ok_or(Error::Invariant("empty dependence name"))?
            .kind
        else {
            return Ok(true);
        };
        let prefix = match self.prefix(name, scope) {
            Ok(prefix) => prefix,
            Err(Error::Missing("dependent qualified name lookup")) => return Ok(true),
            Err(error) => return Err(error),
        };
        if let Some(mut prefix) = prefix {
            while self.graph.scope(prefix).kind != ScopeKind::Namespace {
                let node = self.graph.scope(prefix);
                if node.kind != ScopeKind::Class
                    || node.owner.is_none()
                    || self.dependent_class_scope(prefix)
                {
                    return Ok(true);
                }
                let Some(parent) = node.parent else {
                    return Ok(true);
                };
                prefix = parent;
            }
        }
        let found = if let Some(prefix) = prefix {
            self.qualified_result(prefix, name_key.into(), Filter::Ordinary)?
        } else {
            self.unqualified_result(scope, name_key.into(), Filter::Ordinary)?
        };
        let Some(binding) = found.binding.filter(|_| !found.ambiguous) else {
            return Ok(prefix.is_none() && found.binding.is_none());
        };
        let binding = self.graph.binding(binding);
        if self
            .graph
            .entity(binding.entity)
            .home
            .is_some_and(|scope| self.dependent_class_scope(scope))
        {
            return Ok(true);
        }
        Ok(match binding.kind {
            BindingKind::Variable | BindingKind::Parameter => binding
                .ty
                .is_none_or(|ty| self.exception_dependent_type(ty)),
            BindingKind::Enumerator => false,
            BindingKind::Function if fixed_template_id => false,
            BindingKind::Function => found.functions.iter().any(|&binding| {
                self.graph
                    .binding(binding)
                    .ty
                    .is_some_and(|ty| self.exception_dependent_type(ty))
            }),
            _ => true,
        })
    }
}

use std::collections::HashSet;

impl super::graph::Graph {
    fn type_has_template_owner(&self, entity: super::graph::EntityId) -> bool {
        let node = self.entity(entity);
        let mut scope = node
            .enumeration
            .map(|enumeration| enumeration.owner)
            .or(node.home);
        while let Some(current) = scope {
            let node = self.scope(current);
            if node.kind != ScopeKind::Class {
                break;
            }
            if node
                .owner
                .is_some_and(|owner| self.entity(owner).template_head.is_some())
            {
                return true;
            }
            scope = node.parent;
        }
        false
    }

    pub(super) fn dependent_type(&self, root: TypeId) -> bool {
        let mut work = vec![root];
        let mut visited = HashSet::new();
        while let Some(ty) = work.pop() {
            if !visited.insert(ty) {
                continue;
            }
            match &self.types.get(ty).kind {
                TypeKind::TemplateParameter { .. }
                | TypeKind::Expansion(_)
                | TypeKind::BoundPack { .. } => return true,
                TypeKind::Pack(elements) => work.extend(elements.iter().copied()),
                TypeKind::Class(entity)
                    if self.entity(*entity).template_head.is_some()
                        || self.type_has_template_owner(*entity)
                        || self
                            .class_templates
                            .instance(*entity)
                            .is_some_and(|instance| instance.dependent) =>
                {
                    return true;
                }
                TypeKind::Enum(entity) if self.type_has_template_owner(*entity) => return true,
                TypeKind::Pointer(element) | TypeKind::Reference { element, .. } => {
                    work.push(*element);
                }
                TypeKind::Array {
                    element, operand, ..
                } => {
                    work.push(*element);
                    work.extend(operand);
                }
                TypeKind::MemberPointer { owner, element } => {
                    work.push(*owner);
                    work.push(*element);
                }
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    work.push(*result);
                    work.extend(parameters.iter().copied());
                }
                _ => (),
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn explicit_callee_keys_reuse_signatures_without_body_or_emission_demand() {
        let source = SourceBuffer::new(
            b"template<class T>T make(){return T::missing;} \
              namespace ns{template<class T>T make(){return T::missing;} \
              template<class T>typename T::missing bad();} \
              struct C{template<class T>static T make(){return T::missing;}}; \
              void probe(){make<int>();make<int>();make<>();make<int,long>(); \
              ::make<int>();ns::make<int>();ns::make<int>();C::make<int>(); \
              ns::bad<int>();ns::make<>();ns::make<int,long>();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"callee-keys").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let declarations = tree.decl_lists.get(declarations).to_vec();
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(declarations[3]).kind
        else {
            unreachable!()
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            unreachable!()
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            unreachable!()
        };
        let expressions = tree
            .stmt_lists
            .get(statements)
            .iter()
            .map(|&statement| {
                let ast::StmtKind::Expression(Some(expression)) =
                    tree.statements.get(statement).kind
                else {
                    unreachable!()
                };
                expression
            })
            .collect::<Vec<_>>();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        for &declaration in &declarations[..3] {
            analyzer.declaration(declaration, scope).unwrap();
        }
        let mut resolved = Vec::new();
        for (index, expression) in expressions.into_iter().enumerate() {
            let key = analyzer.operand_key(expression, scope).unwrap().unwrap();
            let entity = key
                .words
                .iter()
                .find_map(|word| {
                    if let KeyWord::ResolvedFunction(entity) = word {
                        Some(*entity)
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    key.words.windows(2).find_map(|words| {
                        if let [
                            KeyWord::Node(super::super::expression_keys::KeyNode::Function),
                            KeyWord::Entity(entity),
                        ] = words
                        {
                            Some(*entity)
                        } else {
                            None
                        }
                    })
                });
            assert_eq!(entity.is_some(), index < 2 || (4..8).contains(&index));
            if let Some(entity) = entity {
                resolved.push(entity);
            }
        }
        assert_eq!(resolved[0], resolved[1]);
        assert_eq!(resolved[0], resolved[2]);
        assert_eq!(resolved[3], resolved[4]);
        for &entity in &resolved {
            assert!(analyzer.graph.lower_function_body(entity).is_none());
        }
        let primary = analyzer
            .graph
            .function_templates
            .primary(resolved[0])
            .unwrap();
        assert!(
            !analyzer
                .graph
                .function_templates
                .body_demanded(analyzer.graph.binding(primary).entity)
        );
        assert!(analyzer.graph.lower_function_body(resolved[0]).is_none());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.body_context.is_none());
    }

    #[test]
    fn fixed_decltype_checks_restore_context_without_function_use() {
        let source = SourceBuffer::new(
            b"struct X{int member;};X source();void use(){source().member;source().missing;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"fixed-decltype").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let declarations = tree.decl_lists.get(declarations).to_vec();
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(declarations[2]).kind
        else {
            unreachable!()
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            unreachable!()
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            unreachable!()
        };
        let expressions: Vec<_> = tree
            .stmt_lists
            .get(statements)
            .iter()
            .map(|&statement| {
                let ast::StmtKind::Expression(Some(expression)) =
                    tree.statements.get(statement).kind
                else {
                    unreachable!()
                };
                expression
            })
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        for &declaration in &declarations[..2] {
            analyzer.declaration(declaration, scope).unwrap();
        }
        analyzer.template_depth = 3;
        analyzer.unevaluated = 2;
        for (index, expression) in expressions.into_iter().enumerate() {
            let result = analyzer.check_fixed_decltype_operands(expression, scope, 0);
            if index == 0 {
                result.unwrap();
            } else {
                assert!(matches!(
                    result,
                    Err(Error::Diagnostic {
                        message: "no member of that name",
                        ..
                    })
                ));
            }
            assert_eq!(analyzer.template_depth, 3);
            assert_eq!(analyzer.unevaluated, 2);
            assert!(analyzer.decltype_call.is_none());
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
        }
    }
}
