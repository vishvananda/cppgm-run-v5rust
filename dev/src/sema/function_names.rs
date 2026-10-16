// Translated from analyzer_declarations.cpp::declared_name/declare_function. See NOTICE.
//! Callable names have typed lookup identity independent of their source label.
use super::{
    Error,
    analyzer::Analyzer,
    declarators::DeclaratorInfo,
    graph::{Binding, BindingId, BindingKind, FunctionProperties, ScopeId, ScopeKind},
    specifiers::{Specifiers, Virtuality},
    types::{Cv, RefKind, Scalar, TypeKind},
};
use crate::{
    lex::{
        identifiers::NameId,
        pp_lexer::Punctuator,
        vocabulary::{Keyword, SimpleToken},
    },
    support::source::SourceOffset,
    syntax::ast::{self, InitializerKind, NameKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SpecialName {
    Operator { token: SimpleToken, array: bool },
    Literal(NameId),
    Destructor,
    // Shared lookup category; each declaration's identity remains MemberKey.
    Conversion,
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
    fn declaration_index_separates_template_shapes_and_keeps_default_patterns() {
        let source = SourceBuffer::new(
            b"int f(int);template<class T>int f(int);\
              template<class T>long f(int);template<class T,class U>int f(int);\
              template<class Renamed>int f(int);\
              template<class T>int g(T,int=2);template<class U>int g(U,int);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"template-index").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let g = cursor.intern_name(b"g").unwrap();
        let expression_count = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let mut graph = analyzer.analyze(declarations).unwrap();
        let functions: Vec<_> = graph.functions_at(graph.global, f.into()).collect();
        assert_eq!(functions.len(), 4);
        let entities: std::collections::HashSet<_> = functions
            .iter()
            .map(|&binding| graph.binding(binding).entity)
            .collect();
        assert_eq!(entities.len(), 4);
        for (index, &binding) in functions.iter().enumerate() {
            let function = graph.binding(binding);
            let head = graph.entity(function.entity).template_head;
            assert_eq!(head.is_none(), index == 0);
            if let Some(head) = head {
                assert_eq!(
                    graph.templates.parameters(head).len(),
                    if index == 3 { 2 } else { 1 }
                );
            }
            assert_eq!(
                graph
                    .namespace_function(graph.global, f.into(), function.ty.unwrap(), head)
                    .unwrap(),
                Some(binding)
            );
            assert!(!graph.function_templates.body_demanded(function.entity));
        }
        let defaults: Vec<_> = graph.functions_at(graph.global, g.into()).collect();
        assert_eq!(defaults.len(), 1);
        let function = graph.entity(graph.binding(defaults[0]).entity);
        let latest = function.function_parameters.unwrap();
        assert!(tree.parameters.get(latest.parameters)[1].default.is_none());
        let default = function.defaults[1].unwrap().expression;
        assert!(graph.expression_facts.get(default).is_err());
        assert!(graph.root_demands.as_slice().is_empty());
        assert_eq!(expression_count, tree.expressions.mark());
        assert_eq!(declaration_count, tree.declarations.mark());
    }

    #[test]
    fn incompatible_template_redeclaration_preserves_published_declaration() {
        for specification in ["noexcept", "noexcept(sizeof(T)==4)"] {
            let source = SourceBuffer::new(
                format!(
                    "template<class T=int>int f(T,int=1){specification};\
                     template<class U=int>int f(U,int=2){};",
                    if specification == "noexcept" {
                        ""
                    } else {
                        "noexcept(sizeof(U)==8)"
                    }
                )
                .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"template-agreement").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let f = cursor.intern_name(b"f").unwrap();
            let expressions = tree.expressions.mark();
            let syntax_declarations = tree.declarations.mark();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let declarations = analyzer.tree.decl_lists.get(declarations).to_vec();
            let global = analyzer.graph.global;
            analyzer.declaration(declarations[0], global).unwrap();
            let binding = analyzer
                .graph
                .functions_at(global, f.into())
                .next()
                .unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            let head = analyzer.graph.entity(entity).template_head;
            let default = analyzer.graph.entity(entity).defaults[1]
                .unwrap()
                .expression;
            assert!(matches!(
                analyzer.declaration(declarations[1], global),
                Err(Error::Diagnostic {
                    message: "redeclaration with an incompatible exception-specification",
                    ..
                })
            ));
            let functions: Vec<_> = analyzer.graph.functions_at(global, f.into()).collect();
            assert_eq!(functions, [binding]);
            assert_eq!(analyzer.graph.entity(entity).template_head, head);
            assert_eq!(
                analyzer.graph.entity(entity).defaults[1]
                    .unwrap()
                    .expression,
                default
            );
            assert!(analyzer.graph.lookup_limit.is_none());
            assert_eq!(analyzer.template_depth, 0);
            assert!(analyzer.template_head.is_none());
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            assert!(analyzer.graph.expression_facts.get(default).is_err());
            assert_eq!(tree.expressions.mark(), expressions);
            assert_eq!(tree.declarations.mark(), syntax_declarations);
        }
    }
}
impl SpecialName {
    pub(super) fn allocation(self) -> bool {
        matches!(self, Self::Operator { token, .. } if matches!(token.as_keyword(), Some(Keyword::New | Keyword::Delete)))
    }
    pub(super) fn deallocation(self) -> bool {
        matches!(self, Self::Operator { token, .. } if token.as_keyword() == Some(Keyword::Delete))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ValueKey {
    Identifier(NameId),
    Special(SpecialName),
}
impl ValueKey {
    pub(super) fn identifier(self) -> Option<NameId> {
        match self {
            Self::Identifier(name) => Some(name),
            Self::Special(_) => None,
        }
    }
    pub(super) fn special(self) -> Option<SpecialName> {
        match self {
            Self::Special(name) => Some(name),
            Self::Identifier(_) => None,
        }
    }
}
impl From<NameId> for ValueKey {
    fn from(name: NameId) -> Self {
        Self::Identifier(name)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Definition {
    Declaration,
    Body,
    Defaulted,
    Deleted,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AllocationOrigin {
    Predeclared,
    Declared,
}
impl Definition {
    pub(super) fn defines(self) -> bool {
        self != Self::Declaration
    }
}
impl Analyzer<'_> {
    // analyzer_declarations.cpp::redeclare_function: agreement precedes the
    // template-head/default merge and publication of the new declaration.
    pub(super) fn agree_function_redeclaration(
        &mut self,
        entity: super::graph::EntityId,
        ty: super::types::TypeId,
        context: super::declarations::ValueScope<'_>,
        specs: Specifiers,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let old = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("function without type"))?;
        let previous = self.graph.entity(entity).template_head;
        if match (context.head, previous) {
            (Some(a), Some(b)) => !self.same_template_shape(a, b),
            (None, None) => false,
            _ => true,
        } {
            return Err(Error::Invariant("mismatched function declaration index"));
        }
        self.check_function_type_agreement(old, ty, location)?;
        if context.definition == Definition::Deleted {
            return Err(Error::at(
                location,
                "deleted definition of a function declared before",
            ));
        }
        if context.definition.defines() && self.graph.entity(entity).defined {
            return Err(Error::at(location, "redefinition of a function"));
        }
        if specs.is_static
            && self.graph.scope(context.home).kind == ScopeKind::Namespace
            && self
                .graph
                .entity(entity)
                .function
                .is_some_and(|props| !props.internal_linkage)
        {
            return Err(Error::at(
                location,
                "a static declaration of a function declared with external linkage",
            ));
        }
        if self.graph.scope(context.home).kind == ScopeKind::Namespace
            && !context.qualified
            && self.graph.entity(entity).builtin.is_none()
            && let Some(properties) = self.graph.entity(entity).function
        {
            self.agree_language(properties.language, location)?;
        }
        if self.graph.entity(entity).constexpr != specs.constexpr {
            return Err(Error::at(
                location,
                "declarations of a function that disagree on constexpr",
            ));
        }
        if context.head.is_some() {
            let (info, limit) = context.function.ok_or(Error::Invariant(
                "template redeclaration without declarator",
            ))?;
            // Compatibility belongs to redeclaration agreement, before a
            // later declaration can add defaults or merge its template head.
            self.record_function_exceptions(entity, info, info.lookup, location, limit)?;
        }
        Ok(())
    }
    pub(super) fn require_function_definition_types(
        &mut self,
        ty: super::types::TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self.graph.types.function_parameter_count(ty)?;
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("definition without function type"));
        };
        for index in 0..=count {
            let ty = if index == count {
                result
            } else {
                self.graph.types.function_parameter(ty, index)?
            };
            if let TypeKind::Class(class) = self.graph.types.get(ty).kind
                && !self.complete_class(class)?
            {
                return Err(Error::at(
                    location,
                    "a parameter or return type of a function definition is incomplete",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn predeclare_allocations(&mut self) -> Result<(), Error> {
        let void = self.graph.types.scalar(Scalar::Void)?;
        let pointer = self.graph.types.pointer(void)?;
        let size = self.graph.types.scalar(Scalar::Basic(
            crate::types::FundamentalType::UnsignedLongInt,
        ))?;
        let allocate = self.graph.types.intern(
            TypeKind::Function {
                result: pointer,
                parameters: vec![size],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let free = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters: vec![pointer],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        for (keyword, ty) in [(Keyword::New, allocate), (Keyword::Delete, free)] {
            for array in [false, true] {
                let name = SpecialName::Operator {
                    token: SimpleToken::keyword(keyword),
                    array,
                };
                let entity = self.graph.add_entity(Some(ty), None, Keyword::Int)?;
                self.graph.entity_mut(entity).function = Some(FunctionProperties {
                    no_inline: false,
                    language: super::linkage::Language::Cpp,
                    denotes: None,
                    internal_linkage: false,
                    is_static: false,
                    special: Some(name),
                    allocation: Some(AllocationOrigin::Predeclared),
                });
                self.graph.entity_mut(entity).exceptions =
                    super::exceptions::ExceptionState::Value(if keyword == Keyword::Delete {
                        super::exceptions::ExceptionSet::None
                    } else {
                        super::exceptions::ExceptionSet::All
                    });
                let binding = self.graph.add_binding(
                    self.graph.global,
                    None,
                    Binding {
                        kind: BindingKind::Function,
                        name: None,
                        entity,
                        ty: Some(ty),
                        key: Keyword::Int,
                    },
                )?;
                self.graph
                    .record_special_value(self.graph.global, name, binding)?;
            }
        }
        Ok(())
    }
    pub(super) fn declaration_key(&self, info: &DeclaratorInfo) -> Result<ValueKey, Error> {
        let name = info
            .syntax_name
            .ok_or(Error::Invariant("declaration without a name"))?;
        self.value_name(name)
    }
    pub(super) fn expression_name(kind: NameKind) -> Result<ValueKey, Error> {
        match kind {
            NameKind::Identifier(name) => Ok(name.into()),
            NameKind::Operator { op, array } => Ok(ValueKey::Special(SpecialName::Operator {
                token: op.token,
                array,
            })),
            NameKind::LiteralOperator(name) => Ok(ValueKey::Special(SpecialName::Literal(name))),
            _ => Err(Error::Missing(
                "operator and special member semantic identities",
            )),
        }
    }
    pub(super) fn value_name(&self, name: ast::Name) -> Result<ValueKey, Error> {
        let node = self.tree.names.get(name);
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty declaration name"))?;
        if last.arguments.is_some() {
            return Err(Error::at(
                node.location,
                "template-ids are not supported here",
            ));
        }
        if let NameKind::Identifier(name) = last.kind {
            return Ok(ValueKey::Identifier(name));
        }
        let kind = match last.kind {
            NameKind::Operator { op, array } => SpecialName::Operator {
                token: op.token,
                array,
            },
            NameKind::LiteralOperator(name) => SpecialName::Literal(name),
            NameKind::Destructor(_) | NameKind::DestructorDecltype(_) => SpecialName::Destructor,
            NameKind::Decltype(_) => {
                return Err(Error::at(
                    node.location,
                    "a decltype-specifier does not name a declaration",
                ));
            }
            NameKind::Conversion(_) => {
                return Err(Error::at(
                    node.location,
                    "a conversion function with a return type",
                ));
            }
            NameKind::Identifier(_) => {
                return Err(Error::Invariant("lost ordinary declaration name"));
            }
        };
        Ok(ValueKey::Special(kind))
    }

    pub(super) fn record_callable_label(
        &mut self,
        binding: BindingId,
        info: &DeclaratorInfo,
        key: ValueKey,
    ) -> Result<(), Error> {
        if key.special().is_some() {
            let name = info
                .syntax_name
                .ok_or(Error::Invariant("special callable without source name"))?;
            self.graph.special_labels.insert(binding, name);
            let entity = self.graph.binding(binding).entity;
            if self.graph.entity(entity).special_name.is_none() {
                self.graph.entity_mut(entity).special_name = Some(name);
            }
        }
        Ok(())
    }

    pub(super) fn declare_function_value(
        &mut self,
        info: &mut DeclaratorInfo,
        specs: Specifiers,
        scope: ScopeId,
        location: SourceOffset,
        definition: Definition,
    ) -> Result<BindingId, Error> {
        self.function_specifier_type(specs, info, scope)?;
        if specs.friendship == super::specifiers::Friendship::Friend
            && (self.function_template_id(info).is_some()
                || info.home.is_some() && self.templated_friend_class(scope))
        {
            return self.declare_friend_specialization(info, specs, scope, location);
        }
        let key = self.declaration_key(info)?;
        let befriending = if specs.friendship == super::specifiers::Friendship::Friend {
            Some(self.friend_owner(scope, location)?)
        } else {
            None
        };
        if let Some(class) = befriending
            && let Some(home) = info.home
        {
            if self.active_template_head(scope).is_some() {
                return Err(Error::Missing("qualified friend templates"));
            }
            let candidate = if self.graph.scope(home).kind == ScopeKind::Namespace {
                self.graph.namespace_function(home, key, info.ty, None)?
            } else {
                let mut next = 0;
                let mut candidate = None;
                while let Some(binding) = self.graph.next_function(home, key, &mut next) {
                    let old = self
                        .graph
                        .binding(binding)
                        .ty
                        .ok_or(Error::Invariant("friend function without type"))?;
                    if self.same_function_parameters(old, info.ty)? {
                        candidate = Some(binding);
                        break;
                    }
                }
                candidate
            };
            if let Some(binding) = candidate
                && self.graph.entity(self.graph.binding(binding).entity).home == Some(home)
            {
                self.graph
                    .friendships
                    .record(class, self.graph.binding(binding).entity);
                return Ok(binding);
            }
            return Err(Error::at(
                location,
                "a qualified friend declaration names no declared function",
            ));
        }
        let home = if befriending.is_some() {
            self.enclosing_namespace(scope)
        } else {
            info.home.unwrap_or(self.declaration_scope(scope))
        };
        if info.home.is_some() && specs.is_static && self.graph.scope(home).kind == ScopeKind::Class
        {
            return Err(Error::at(
                location,
                "static on a member function's declaration outside its class",
            ));
        }
        let head = self.active_template_head(scope);
        if head.is_some() {
            if self.graph.scope(home).kind == ScopeKind::Class
                && specs.virtuality == Virtuality::Virtual
            {
                return Err(Error::at(location, "a virtual member function template"));
            }
            if key == ValueKey::Special(SpecialName::Destructor) {
                return Err(Error::at(location, "a destructor declared as a template"));
            }
        }
        self.check_member_function_redeclaration(info, home, head, location)?;
        if info.home.is_some() {
            let matching = if self.graph.scope(home).kind == ScopeKind::Namespace {
                self.graph
                    .namespace_function(home, key, info.ty, head)?
                    .is_some()
            } else {
                let mut next = 0;
                let mut matching = false;
                while let Some(candidate) = self.graph.next_function(home, key, &mut next) {
                    if self.graph.entity(self.graph.binding(candidate).entity).home == Some(home)
                        && self.same_declared_function(candidate, info.ty, head)?
                    {
                        matching = true;
                        break;
                    }
                }
                matching
            };
            if !matching {
                return Err(Error::at(
                    location,
                    "qualified definition does not match a declaration",
                ));
            }
        }
        if info.home.is_some() && self.graph.value_binding(home, key).is_none() {
            return Err(Error::at(
                location,
                "qualified definition does not match a declaration",
            ));
        }
        self.check_callable_qualifiers(info, key, specs, home, location)?;
        self.check_member_overload(info.ty, key, specs, home, location)?;
        let exception_limit = self.graph.binding_limit();
        let binding = self.declare_value(
            key,
            info.ty,
            super::declarations::ValueScope {
                qualified: info.home.is_some(),
                home,
                print: if befriending.is_some() {
                    home
                } else {
                    info.home.unwrap_or(scope)
                },
                head,
                function: Some((info, exception_limit)),
                definition,
                declarator_location: location,
            },
            BindingKind::Function,
            location,
            specs,
        )?;
        let entity = self.graph.binding(binding).entity;
        if head.is_some() && self.graph.function_origin(entity)? == binding {
            // Redeclarations already checked their specification in agreement.
            // A first declaration records it once, before default name checks.
            self.record_template_result_order(entity, info);
            self.record_function_exceptions(entity, info, info.lookup, location, exception_limit)?;
        }
        self.record_callable_label(binding, info, key)?;
        if self.semantic() {
            self.check_operator_declaration(binding, location)?;
        }
        if let Some(class) = befriending {
            let entity = self.graph.binding(binding).entity;
            self.graph.friendships.record(class, entity);
            if self.graph.friendships.hidden(binding) {
                self.graph.friendships.index(class, key, binding);
            }
            if definition == Definition::Body {
                self.graph.entity_mut(entity).inline = super::specifiers::InlinePreference::Inline;
            }
        }
        if self.graph.scope(scope).kind == ScopeKind::Class
            && befriending.is_none()
            && head.is_none()
            && !matches!(definition, Definition::Defaulted | Definition::Deleted)
            && matches!(key, ValueKey::Special(SpecialName::Operator { token, array: false }) if token.as_punctuator() == Some(Punctuator::Assign))
        {
            let ty = self.graph.types.canonical_function(info.ty)?;
            if self.graph.types.function_parameter_count(ty)? == 1
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Function {
                        variadic: false,
                        ..
                    }
                )
            {
                let parameter = self.graph.types.function_parameter(ty, 0)?;
                let parameter = if let TypeKind::Reference { element, .. } =
                    self.graph.types.get(parameter).kind
                {
                    element
                } else {
                    parameter
                };
                if let TypeKind::Class(class) = self.graph.types.get(parameter).kind
                    && self.graph.scope(scope).owner == Some(class)
                {
                    self.graph.entity_mut(class).layout_category =
                        super::layout::LayoutCategory::NonPod;
                }
            }
        }
        let entity = self.graph.binding(binding).entity;
        if !self.graph.entity(entity).defined || definition == Definition::Body {
            self.graph.entity_mut(entity).function_parameters = info.parameters;
        }
        if definition == Definition::Deleted {
            self.graph.entity_mut(entity).availability =
                super::graph::FunctionAvailability::Deleted;
        }
        if self.semantic()
            && self.template_depth == 0
            && let Some(clause) = info.parameters
        {
            self.record_function_defaults(entity, clause, info.lookup, info.ty)?;
        } else if self.semantic()
            && self.template_depth != 0
            && let Some(clause) = info.parameters
        {
            self.record_template_function_defaults(entity, clause, info.lookup, info.ty)?;
        }
        Ok(binding)
    }

    fn check_callable_qualifiers(
        &self,
        info: &DeclaratorInfo,
        key: ValueKey,
        specs: Specifiers,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let TypeKind::Function { cv, reference, .. } = self.graph.types.get(info.ty).kind else {
            return Err(Error::Invariant("callable without function type"));
        };
        if self.graph.scope(home).kind != ScopeKind::Class
            || specs.is_static
            || key == ValueKey::Special(SpecialName::Destructor)
        {
            if reference.is_some() {
                return Err(Error::at(
                    location,
                    "a ref-qualifier on a function that is not a non-static member function",
                ));
            }
            if cv.0 != 0 {
                return Err(Error::at(
                    location,
                    "a cv-qualifier on a function that is not a non-static member function",
                ));
            }
        }
        Ok(())
    }

    fn check_member_overload(
        &mut self,
        ty: super::types::TypeId,
        key: ValueKey,
        specs: Specifiers,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.scope(home).kind != ScopeKind::Class
            || self.active_template_head(home).is_some()
        {
            return Ok(());
        }
        let ty = self.graph.types.canonical_function(ty)?;
        let TypeKind::Function { reference, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("member overload without function type"));
        };
        // Probe only the named overload set; no unrelated declaration or
        // body is visited. Return types do not distinguish overloads.
        let mut next = 0;
        while let Some(binding) = self.graph.next_function(home, key, &mut next) {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.template_head.is_some() || entity.home != Some(home) {
                continue;
            }
            let other = entity.ty.ok_or(Error::Invariant("function without type"))?;
            let is_static = entity.function.is_some_and(|props| props.is_static);
            let other = self.graph.types.canonical_function(other)?;
            // An existing signature follows redeclare_function, retaining
            // its effective static property (an out-of-class definition does
            // not repeat `static`). Only a new overload runs this check.
            if self.same_function_parameters(ty, other)? {
                return Ok(());
            }
            let TypeKind::Function {
                reference: old_ref, ..
            } = self.graph.types.get(other).kind
            else {
                return Err(Error::Invariant("member overload without function type"));
            };
            if !self.same_member_parameter_list(ty, other)? {
                continue;
            }
            if is_static != specs.is_static {
                return Err(Error::at(
                    location,
                    "a static and a non-static member function with the same parameters",
                ));
            }
            if reference.is_none() != old_ref.is_none() {
                return Err(Error::at(
                    location,
                    "member functions with the same parameters, some with a ref-qualifier and some without",
                ));
            }
        }
        Ok(())
    }

    fn same_member_parameter_list(
        &self,
        a: super::types::TypeId,
        b: super::types::TypeId,
    ) -> Result<bool, Error> {
        let (
            TypeKind::Function {
                parameters: ap,
                variadic: av,
                ..
            },
            TypeKind::Function {
                parameters: bp,
                variadic: bv,
                ..
            },
        ) = (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
        else {
            return Err(Error::Invariant(
                "member parameter comparison without functions",
            ));
        };
        Ok(ap == bp && av == bv)
    }

    pub(super) fn declare_function_init(
        &mut self,
        mut info: DeclaratorInfo,
        specs: Specifiers,
        init: ast::InitDeclarator,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let limit = self.graph.binding_limit();
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        let deleted = init
            .initializer
            .is_some_and(|init| matches!(init.kind, InitializerKind::Delete));
        let defaulted = init
            .initializer
            .is_some_and(|init| matches!(init.kind, InitializerKind::Default));
        let definition = if deleted {
            Definition::Deleted
        } else if defaulted {
            Definition::Defaulted
        } else {
            Definition::Declaration
        };
        let binding = self.declare_function_value(&mut info, specs, scope, location, definition)?;
        if specs.friendship == super::specifiers::Friendship::Friend
            && (info.home.is_some() || self.function_template_id(&info).is_some())
        {
            if self.function_template_id(&info).is_some() {
                self.pure_specifier(
                    self.graph.binding(binding).entity,
                    init.initializer,
                    &info,
                    specs,
                    scope,
                    location,
                )?;
                if defaulted {
                    return Err(Error::at(
                        location,
                        "only special member functions can be defaulted",
                    ));
                }
            }
            super::record(
                &mut self.graph.declared_bindings,
                init.declarator.index(),
                binding,
            );
            return Ok(());
        }
        let key = self.declaration_key(&info)?;
        let entity = self.graph.binding(binding).entity;
        self.graph.entity_mut(entity).constexpr |= specs.constexpr;
        if self.graph.entity(entity).template_head.is_none() {
            self.record_function_exceptions(entity, &info, info.lookup, location, limit)?;
        }
        self.virtual_declaration(binding, &info, specs, location)?;
        self.pure_specifier(entity, init.initializer, &info, specs, scope, location)?;
        self.settle_declaration_access(Some(binding))?;
        if defaulted {
            self.default_assignment(&info, key, home, location)?;
            if self.semantic()
                && self
                    .graph
                    .entity(entity)
                    .defaults
                    .iter()
                    .any(Option::is_some)
            {
                return Err(Error::at(
                    location,
                    "an explicitly defaulted function has a default argument",
                ));
            }
            let first = self.graph.function_origin(entity)? == binding;
            self.graph.entity_mut(entity).special_definition =
                Some(super::special_members::SpecialDefinition::Defaulted { first, body: None });
            self.graph.entity_mut(entity).assignment_exceptions =
                if first && !self.declarator_specifies_exceptions(Some(init.declarator)) {
                    super::assignment::ExceptionSource::Subobjects
                } else {
                    super::assignment::ExceptionSource::Explicit
                };
            if first {
                self.graph.entity_mut(entity).definition_emission =
                    super::function_demands::DefinitionEmission::Demanded;
            }
            if self.semantic()
                && self
                    .graph
                    .entity(
                        self.graph
                            .scope(home)
                            .owner
                            .ok_or(Error::Invariant("assignment without class"))?,
                    )
                    .complete
            {
                self.prepare_default_assignment(binding)?;
                if !first
                    && self
                        .graph
                        .entity(entity)
                        .assignment
                        .as_ref()
                        .is_some_and(|p| p.deleted)
                {
                    return Err(Error::at(
                        location,
                        "an explicitly defaulted function is defined as deleted",
                    ));
                }
            }
            if self.lowering() && !first {
                self.graph.boundary_roots.push(entity);
            }
        }
        self.graph.entity_mut(entity).defined |= deleted || defaulted;
        super::record(
            &mut self.graph.declared_bindings,
            init.declarator.index(),
            binding,
        );
        Ok(())
    }

    fn default_assignment(
        &self,
        info: &DeclaratorInfo,
        key: ValueKey,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let class = self.graph.scope(home).owner;
        let TypeKind::Function {
            result,
            ref parameters,
            ..
        } = self.graph.types.get(info.ty).kind
        else {
            return Err(Error::Invariant("defaulted function without type"));
        };
        let assign = matches!(key, ValueKey::Special(SpecialName::Operator { token, array: false }) if token.as_punctuator() == Some(Punctuator::Assign));
        let matching_class = |ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(entity) if Some(entity) == class);
        let special = assign
            && self.graph.scope(home).kind == ScopeKind::Class
            && parameters.len() == 1
            && matches!(self.graph.types.get(result).kind, TypeKind::Reference { element, kind: RefKind::Lvalue } if matching_class(element) && self.graph.types.get(element).cv.0 == 0)
            && matches!(self.graph.types.get(parameters[0]).kind, TypeKind::Reference { element, .. } if matching_class(element));
        if !special {
            return Err(Error::at(
                location,
                "only special member functions can be defaulted",
            ));
        }
        // Types mode checks names in defaults, but never constructs the
        // semantic default-argument facts this C++ path tests.
        Ok(())
    }
}
