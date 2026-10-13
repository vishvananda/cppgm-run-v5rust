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
}
impl SpecialName {
    pub(super) fn allocation(self) -> bool {
        matches!(self, Self::Operator { token, .. } if matches!(token.as_keyword(), Some(Keyword::New | Keyword::Delete)))
    }
    pub(super) fn deallocation(self) -> bool {
        matches!(self, Self::Operator { token, .. } if token.as_keyword() == Some(Keyword::Delete))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
                    .record_special_value(self.graph.global, name, binding);
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
        let key = self.declaration_key(info)?;
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        if self.semantic() && self.graph.scope(home).kind == ScopeKind::Class {
            return Err(Error::Missing("class semantic functions"));
        }
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
        self.check_member_function_redeclaration(info, home, location)?;
        if info.home.is_some() && self.graph.value_binding(home, key).is_none() {
            return Err(Error::at(
                location,
                "qualified definition does not match a declaration",
            ));
        }
        self.check_callable_qualifiers(info, key, specs, home, location)?;
        let binding = self.declare_value(
            key,
            info.ty,
            super::declarations::ValueScope {
                home,
                print: info.home.unwrap_or(scope),
                head,
                definition,
            },
            BindingKind::Function,
            location,
            specs,
        )?;
        self.record_callable_label(binding, info, key)?;
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

    pub(super) fn declare_function_init(
        &mut self,
        mut info: DeclaratorInfo,
        specs: Specifiers,
        init: ast::InitDeclarator,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let limit = self.graph.binding_limit();
        let key = self.declaration_key(&info)?;
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
        let entity = self.graph.binding(binding).entity;
        self.graph.entity_mut(entity).constexpr |= specs.constexpr;
        self.record_function_exceptions(entity, &info, info.lookup, location, limit)?;
        if defaulted {
            self.default_assignment(&info, key, home, location)?;
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
