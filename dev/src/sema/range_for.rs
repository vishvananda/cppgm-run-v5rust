// Translated from sema/analyzer_range_for.cpp. See NOTICE.
//! Semantic loop equivalents own implicit variables and typed expression IDs.
//! Written ranges are analyzed once; lowering consumes this sparse statement fact.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ExpressionDependence, ExpressionOrigin, ValueCategory as V},
    function_names::ValueKey,
    graph::{BindingKind, EntityId, Graph, ScopeId},
    lookup::{Filter, LookupResult},
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, pp_lexer::Punctuator as P, vocabulary::SimpleToken},
    support::source::SourceOffset,
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
enum RangeVariable {
    Range,
    Begin,
    End,
    Index,
}
impl RangeVariable {
    fn names(self) -> (&'static [u8], &'static [u8]) {
        match self {
            Self::Range => (b"<range>", b"__range"),
            Self::Begin => (b"<begin>", b"__begin"),
            Self::End => (b"<end>", b"__end"),
            Self::Index => (b"<index>", b"__idx"),
        }
    }
}
#[derive(Clone, Copy)]
struct RangeContext {
    scope: ScopeId,
    location: SourceOffset,
    dependent: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct RangeForFacts {
    pub range: Option<ast::InitDeclarator>,
    pub begin: ast::InitDeclarator,
    pub end: Option<ast::InitDeclarator>,
    pub declaration: ast::InitDeclarator,
    pub condition: ast::ExprId,
    pub increment: ast::ExprId,
}
#[derive(Default)]
pub(super) struct RangeLoops {
    records: HashMap<ast::StmtId, RangeForFacts>,
    objects: HashSet<EntityId>,
}
impl Graph {
    pub(crate) fn lower_range_for(&self, id: ast::StmtId) -> Result<RangeForFacts, Error> {
        self.range_loops
            .records
            .get(&id)
            .copied()
            .ok_or(Error::Invariant("range-for without equivalent"))
    }
    pub(crate) fn lower_range_object(&self, entity: EntityId) -> bool {
        self.range_loops.objects.contains(&entity)
    }
}
impl Analyzer<'_> {
    fn range_name(&mut self, spelling: &[u8], location: SourceOffset) -> Result<ast::Name, Error> {
        let identifier = self
            .identifiers
            .intern(spelling)
            .map_err(|_| Error::Limit("implicit range name"))?;
        self.tree
            .implicit_name(identifier, location)
            .map_err(|_| Error::Limit("implicit range syntax storage limit"))
    }
    fn range_expression(
        &mut self,
        kind: ExprKind,
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        self.tree
            .expressions
            .add(ast::Expression { location, kind })
            .map_err(|_| Error::Limit("implicit range syntax storage limit"))
    }
    fn range_operator(punctuator: P) -> Result<ast::Operator, Error> {
        Ok(ast::Operator {
            token: SimpleToken::punctuator(punctuator)
                .ok_or(Error::Invariant("implicit range operator"))?,
            alternative: false,
        })
    }
    fn range_binary(
        &mut self,
        op: P,
        left: ast::ExprId,
        right: ast::ExprId,
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        self.range_expression(
            ExprKind::Binary {
                op: Self::range_operator(op)?,
                left,
                right,
                assignment: false,
            },
            location,
        )
    }
    fn range_unary(
        &mut self,
        op: P,
        operand: ast::ExprId,
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        self.range_expression(
            ExprKind::Unary {
                op: Self::range_operator(op)?,
                operand,
            },
            location,
        )
    }
    fn range_constant(
        &mut self,
        ty: TypeId,
        value: u64,
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        let name = self.range_name(b"__idx", location)?;
        let id = self.range_expression(ExprKind::Id(name), location)?;
        let mut facts = ExprFacts::new(ty, V::Prvalue);
        facts.folded = Some(value);
        facts.origin = ExpressionOrigin::Implicit;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), ty);
        Ok(id)
    }
    fn range_variable(
        &mut self,
        variable: RangeVariable,
        ty: TypeId,
        source: ast::ExprId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<ast::InitDeclarator, Error> {
        let (key, label) = variable.names();
        let syntax_name = self.range_name(key, location)?;
        let declarator = self
            .tree
            .implicit_declarator(syntax_name, location)
            .map_err(|_| Error::Limit("implicit range syntax storage limit"))?;
        let name = self.declarator_name(Some(declarator))?;
        let init = ast::InitDeclarator {
            declarator,
            initializer: Some(ast::Initializer {
                location,
                kind: ast::InitializerKind::Assign(source),
            }),
            tokens: ast::TokenSpan::default(),
        };
        let specs = super::specifiers::Specifiers::implicit(ty, self.dependent_uses.get());
        let info = super::declarators::DeclaratorInfo {
            ty,
            name,
            home: None,
            lookup: scope,
            parameters: None,
            qualifiers: None,
            syntax_name: Some(syntax_name),
        };
        self.declare_analyzed_init(specs, init, scope, &mut None, info)?;
        let binding = self.graph.declared_bindings[declarator.index()]
            .ok_or(Error::Invariant("implicit range declaration"))?;
        let entity = self.graph.binding(binding).entity;
        let printed = self
            .identifiers
            .intern(label)
            .map_err(|_| Error::Limit("implicit range label"))?;
        self.graph.entity_mut(entity).name = Some(printed);
        Ok(init)
    }
    fn range_use(
        &mut self,
        range: Option<ast::InitDeclarator>,
        named: Option<ast::ExprId>,
        dependent: bool,
        location: SourceOffset,
    ) -> Result<ast::ExprId, Error> {
        let (name, mut facts) = if let Some(named) = named {
            let root = self.graph.expression_facts.root(named);
            let ExprKind::Id(name) = self.tree.expressions.get(root).kind else {
                return Err(Error::Invariant("named range is not an identifier"));
            };
            (name, self.graph.expression_facts.get(root)?)
        } else {
            let declaration = range.ok_or(Error::Invariant("range use without declaration"))?;
            let binding = self.graph.declared_bindings[declaration.declarator.index()]
                .ok_or(Error::Invariant("range use without binding"))?;
            let variable = self.graph.entity(self.graph.binding(binding).entity);
            let ty = variable
                .ty
                .ok_or(Error::Invariant("range use without type"))?;
            let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                element
            } else {
                ty
            };
            let label = variable
                .name
                .ok_or(Error::Invariant("range use without name"))?;
            let name = self
                .tree
                .implicit_name(label, location)
                .map_err(|_| Error::Limit("implicit range syntax storage limit"))?;
            super::record(&mut self.graph.resolved_names, name.index(), binding);
            let mut facts = ExprFacts::new(ty, V::Lvalue);
            if dependent {
                facts.dependence = ExpressionDependence::Dependent;
            }
            (name, facts)
        };
        facts.conversion = None;
        facts.origin = ExpressionOrigin::Implicit;
        let id = self.range_expression(ExprKind::Id(name), location)?;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), facts.ty);
        Ok(id)
    }
    fn mark_range_expression(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let mut facts = self.graph.expression_facts.get(id)?;
        facts.origin = ExpressionOrigin::Implicit;
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }
    fn range_endpoint(
        &mut self,
        name: NameId,
        receiver: ast::ExprId,
        members: bool,
        context: RangeContext,
    ) -> Result<ast::ExprId, Error> {
        let RangeContext {
            dependent,
            scope,
            location,
        } = context;
        let name_syntax = self
            .tree
            .implicit_name(name, location)
            .map_err(|_| Error::Limit("implicit range syntax storage limit"))?;
        let callee = if members {
            self.range_expression(
                ExprKind::Member {
                    base: receiver,
                    op: Self::range_operator(P::Dot)?,
                    member: name_syntax,
                },
                location,
            )?
        } else {
            self.range_expression(ExprKind::Id(name_syntax), location)?
        };
        let arguments = if members {
            List::default()
        } else {
            let start = self.tree.expr_lists.begin();
            self.tree.expr_lists.push(receiver);
            self.tree
                .expr_lists
                .finish(start)
                .map_err(|_| Error::Limit("implicit range syntax storage limit"))?
        };
        let call = self.range_expression(ExprKind::Call { callee, arguments }, location)?;
        if members {
            self.typed_expression(call, scope)?;
        } else {
            let mut found = LookupResult::default();
            self.argument_dependent_lookup(
                ValueKey::Identifier(name),
                arguments,
                &mut found,
                super::call_lookup::AssociationContext::from_dependence(dependent),
            )?;
            if found.functions.as_slice().is_empty() {
                return Err(Error::at(
                    location,
                    "no begin or end function for the range (6.5.4p1: argument-dependent lookup)",
                ));
            }
            let facts = self.named_expression(callee, &found, scope)?;
            self.graph.expression_facts.set(callee, facts);
            super::record(&mut self.graph.expression_scopes, callee.index(), scope);
            let saved = self.pretyped_callable.replace(callee);
            let result = self.typed_expression(call, scope);
            self.pretyped_callable = saved;
            result?;
        }
        self.mark_range_expression(call)?;
        Ok(call)
    }
    pub(super) fn range_declarator(
        &mut self,
        specs: super::specifiers::Specifiers,
        declarator: ast::DeclaratorId,
        range: ast::ExprId,
        block: ScopeId,
        dependent: bool,
        location: SourceOffset,
    ) -> Result<super::declarators::DeclaratorInfo, Error> {
        let mut info = self.placeholder_declarator(Some(declarator), specs, block)?;
        if !self.holds_auto(info.ty) {
            return Ok(info);
        }
        let list_element =
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(range).kind {
                Some(self.deduce_list_range(elements, block, location)?)
            } else {
                None
            };
        if let Some(super::initializer_lists::RangeElement::Dependent(ty)) = list_element {
            info.ty = ty;
            return Ok(info);
        }
        let element =
            if let Some(super::initializer_lists::RangeElement::Concrete(ty)) = list_element {
                Some((ty, V::Lvalue))
            } else {
                let mut range_type = self.decltype_type(range, block)?;
                if let TypeKind::Reference { element, .. } = self.graph.types.get(range_type).kind {
                    range_type = element;
                }
                if let TypeKind::Array { element, .. } = self.graph.types.get(range_type).kind {
                    Some((element, V::Lvalue))
                } else if self.template_depth == 0 {
                    return Err(Error::at(location, "unsupported range-based for deduction"));
                } else if self.semantic() && self.body_context.is_some() {
                    self.known_range_element(range, block, dependent, location)?
                } else {
                    None
                }
            };
        if let Some((element, category)) = element {
            let auto = self.graph.types.intern(TypeKind::Auto, Cv::default())?;
            let pattern = self.replace_auto(info.ty, auto, specs.auto_cv)?;
            let mut arguments = [None];
            if !self.deduce_call_argument(pattern, element, category, 0, &mut arguments)?
                || arguments[0].is_none()
            {
                return Err(Error::at(
                    location,
                    "auto deduction fails for the range's element (7.1.6.4p6)",
                ));
            }
            info.ty = self.replace_auto(
                info.ty,
                arguments[0].ok_or(Error::Invariant("range element deduction"))?,
                specs.auto_cv,
            )?;
        } else {
            info.ty = self.dependent_operand(range, block, false)?;
        }
        Ok(info)
    }
    fn known_range_member(
        &mut self,
        range: ast::ExprId,
        block: ScopeId,
        location: SourceOffset,
    ) -> Result<Option<ast::ExprId>, Error> {
        let Some(function) = self.body_context.as_ref().map(|body| body.entity) else {
            return Ok(None);
        };
        let function = self.graph.entity(function);
        let Some(home) = function.home else {
            return Ok(None);
        };
        let Some(class) = self.graph.scope(home).owner else {
            return Ok(None);
        };
        let Some(properties) = function.function else {
            return Ok(None);
        };
        let TypeKind::Function { cv, .. } = self
            .graph
            .types
            .get(function.ty.ok_or(Error::Invariant("range function type"))?)
            .kind
        else {
            return Err(Error::Invariant("range function signature"));
        };
        let mut root = range;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(root).kind {
            root = inner;
        }
        let (name, arrow) = match self.tree.expressions.get(root).kind {
            ExprKind::Id(name) => (name, false),
            ExprKind::Member { base, op, member } if op.token.as_punctuator() == Some(P::Arrow) => {
                let mut base = base;
                while let ExprKind::Paren(inner) = self.tree.expressions.get(base).kind {
                    base = inner;
                }
                if !matches!(
                    self.tree.expressions.get(base).kind,
                    ExprKind::Keyword(crate::lex::vocabulary::Keyword::This)
                ) {
                    return Ok(None);
                }
                (member, true)
            }
            _ => return Ok(None),
        };
        let syntax = self.tree.names.get(name);
        let components = self.tree.name_lists.get(syntax.components);
        if syntax.global || components.len() != 1 || components[0].arguments.is_some() {
            return Ok(None);
        }
        let ast::NameKind::Identifier(identifier) = components[0].kind else {
            return Ok(None);
        };
        let uses = self.dependent_uses.get();
        let found = if arrow {
            self.qualified_result(home, identifier.into(), Filter::Ordinary)
        } else {
            self.unqualified_result(block, identifier.into(), Filter::Ordinary)
        };
        self.dependent_uses.set(uses);
        let found = found?;
        let Some(binding) = found.binding.filter(|_| !found.ambiguous) else {
            return Ok(None);
        };
        let bound = self.graph.binding(binding);
        let variable = self.graph.entity(bound.entity);
        let Some(ty) = variable.ty else {
            return Ok(None);
        };
        let Some(owner) = variable.home.and_then(|home| self.graph.scope(home).owner) else {
            return Ok(None);
        };
        if bound.kind != BindingKind::Variable || self.graph.dependent_type(ty) {
            return Ok(None);
        }
        let instance = variable.data_member == Some(super::graph::DataMemberKind::Instance);
        if instance
            && (properties.is_static
                || variable.bit_field.is_some()
                || owner != class && !self.graph.is_base_of(owner, class))
        {
            return Ok(None);
        }
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            self.graph.types.qualify(
                ty,
                if instance && variable.mutability != super::graph::Mutability::Mutable {
                    cv
                } else {
                    Cv::default()
                },
            )?
        };
        let name = self
            .tree
            .implicit_name(identifier, location)
            .map_err(|_| Error::Limit("implicit range syntax storage limit"))?;
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        let use_id = self.range_expression(ExprKind::Id(name), location)?;
        let mut facts = ExprFacts::new(ty, V::Lvalue);
        facts.origin = ExpressionOrigin::Implicit;
        self.graph.expression_facts.set(use_id, facts);
        super::record(&mut self.graph.expression_types, use_id.index(), ty);
        Ok(Some(use_id))
    }
    fn known_range_element(
        &mut self,
        range: ast::ExprId,
        block: ScopeId,
        dependent: bool,
        location: SourceOffset,
    ) -> Result<Option<(TypeId, V)>, Error> {
        let member = if dependent || self.expression_type_dependence(range, block, true)? {
            let Some(member) = self.known_range_member(range, block, location)? else {
                return Ok(None);
            };
            Some(member)
        } else {
            None
        };
        let depth = self.template_depth;
        self.template_depth = 0;
        self.unevaluated += 1;
        let result = (|| {
            let range = member.unwrap_or(range);
            let facts = self.typed_expression(range, block)?;
            if let TypeKind::Array { element, .. } = self.graph.types.get(facts.ty).kind {
                return Ok(Some((element, V::Lvalue)));
            }
            let Some(class) = self.graph.class_of(facts.ty) else {
                return Ok(None);
            };
            self.complete_class(class)?;
            if !self.graph.entity(class).defined {
                return Ok(None);
            }
            self.mark_range_expression(range)?;
            let class_scope = self
                .graph
                .entity(class)
                .scope
                .ok_or(Error::Invariant("range class without scope"))?;
            let begin_name = self
                .identifiers
                .intern(b"begin")
                .map_err(|_| Error::Limit("range begin name"))?;
            let end_name = self
                .identifiers
                .intern(b"end")
                .map_err(|_| Error::Limit("range end name"))?;
            let members = self
                .qualified(class_scope, begin_name, Filter::Ordinary)?
                .is_some()
                || self
                    .qualified(class_scope, end_name, Filter::Ordinary)?
                    .is_some();
            let begin = self.range_endpoint(
                begin_name,
                range,
                members,
                RangeContext {
                    scope: block,
                    location,
                    dependent: false,
                },
            )?;
            let element = self.range_unary(P::Multiply, begin, location)?;
            let facts = self.typed_expression(element, block)?;
            Ok(Some((facts.ty, facts.category)))
        })();
        self.unevaluated -= 1;
        self.template_depth = depth;
        result
    }
    pub(super) fn range_for_equivalent(
        &mut self,
        id: ast::StmtId,
        specifiers: List<ast::Specifier>,
        declarator: ast::DeclaratorId,
        source: ast::ExprId,
        block: ScopeId,
    ) -> Result<(), Error> {
        let location = self.tree.statements.get(id).location;
        let uses = self.dependent_uses.get();
        let braced = matches!(
            self.tree.expressions.get(source).kind,
            ExprKind::Braced { .. }
        );
        let braced_range = if braced {
            let auto = self.graph.types.intern(TypeKind::Auto, Cv::default())?;
            let reference = self.graph.types.reference(auto, RefKind::Rvalue)?;
            Some(self.range_variable(RangeVariable::Range, reference, source, block, location)?)
        } else {
            None
        };
        let typed = if braced {
            self.graph.expression_facts.get(source)?
        } else {
            self.typed_expression(source, block)?
        };
        self.mark_range_expression(source)?;
        let root = self.graph.expression_facts.root(source);
        let named = if let ExprKind::Id(name) = self.tree.expressions.get(root).kind {
            self.graph
                .resolved_name(name)
                .filter(|&binding| {
                    let bound = self.graph.binding(binding);
                    matches!(bound.kind, BindingKind::Variable | BindingKind::Parameter)
                        && typed.anonymous_member.is_none()
                        && self.graph.entity(bound.entity).data_member
                            != Some(super::graph::DataMemberKind::Instance)
                })
                .map(|_| source)
        } else {
            None
        };
        let auto = self.graph.types.intern(TypeKind::Auto, Cv::default())?;
        let range = if braced_range.is_some() {
            braced_range
        } else if named.is_none() {
            let reference = self.graph.types.reference(auto, RefKind::Rvalue)?;
            Some(self.range_variable(RangeVariable::Range, reference, source, block, location)?)
        } else {
            None
        };
        let ty = typed.ty;
        let dependent = self.dependent_uses.get() != uses;
        let builtin = self.graph.class_of(ty).filter(|&class| {
            self.graph.entity(class).representation
                == crate::sema::graph::ClassRepresentation::BuiltinInitializerList
        });
        let (begin, end, condition, increment, element) = if let Some(class) = builtin {
            let index_type = self
                .graph
                .types
                .scalar(Scalar::Basic(crate::types::FundamentalType::Int))?;
            let mut members = Vec::with_capacity(2);
            for member in self.graph.entity(class).members.clone() {
                let base = self.range_use(range, named, dependent, location)?;
                let name = self
                    .graph
                    .entity(member)
                    .name
                    .ok_or(Error::Invariant("builtin list member name"))?;
                let name = self.range_name(self.identifiers.spelling(name).as_ref(), location)?;
                members.push(self.range_expression(
                    ExprKind::Member {
                        base,
                        op: Self::range_operator(P::Dot)?,
                        member: name,
                    },
                    location,
                )?);
            }
            let zero = self.range_constant(index_type, 0, location)?;
            let begin =
                self.range_variable(RangeVariable::Index, index_type, zero, block, location)?;
            let left = self.range_use(Some(begin), None, false, location)?;
            let condition = self.range_binary(P::Less, left, members[1], location)?;
            let operand = self.range_use(Some(begin), None, false, location)?;
            let increment = self.range_unary(P::Increment, operand, location)?;
            let index = self.range_use(Some(begin), None, false, location)?;
            let element = self.range_expression(
                ExprKind::Subscript {
                    base: members[0],
                    index,
                },
                location,
            )?;
            (begin, None, condition, increment, element)
        } else if let TypeKind::Array { bound, .. } = self.graph.types.get(ty).kind {
            if bound == 0 {
                return Err(Error::at(
                    location,
                    "a range-based for over an array of unknown bound",
                ));
            }
            let index_type = self.graph.types.scalar(if bound <= 0x7fff_ffff {
                Scalar::Basic(crate::types::FundamentalType::Int)
            } else {
                Scalar::Basic(crate::types::FundamentalType::LongInt)
            })?;
            let zero = self.range_constant(index_type, 0, location)?;
            let begin =
                self.range_variable(RangeVariable::Index, index_type, zero, block, location)?;
            let left = self.range_use(Some(begin), None, false, location)?;
            let right = self.range_constant(index_type, bound, location)?;
            let condition = self.range_binary(P::Less, left, right, location)?;
            let operand = self.range_use(Some(begin), None, false, location)?;
            let increment = self.range_unary(P::Increment, operand, location)?;
            let base = self.range_use(range, named, dependent, location)?;
            let index = self.range_use(Some(begin), None, false, location)?;
            let element = self.range_expression(ExprKind::Subscript { base, index }, location)?;
            (begin, None, condition, increment, element)
        } else {
            let class = self.graph.class_of(ty).ok_or_else(|| {
                Error::at(
                    location,
                    "a range-based for over a range that is no array or class",
                )
            })?;
            self.complete_class(class)?;
            if !self.graph.entity(class).defined {
                return Err(Error::at(
                    location,
                    "a range-based for over an incomplete class",
                ));
            }
            let class_scope = self
                .graph
                .entity(class)
                .scope
                .ok_or(Error::Invariant("range class without scope"))?;
            let begin_name = self
                .identifiers
                .intern(b"begin")
                .map_err(|_| Error::Limit("range begin name"))?;
            let end_name = self
                .identifiers
                .intern(b"end")
                .map_err(|_| Error::Limit("range end name"))?;
            let members = self
                .qualified(class_scope, begin_name, Filter::Ordinary)?
                .is_some()
                || self
                    .qualified(class_scope, end_name, Filter::Ordinary)?
                    .is_some();
            let receiver = self.range_use(range, named, dependent, location)?;
            let first = self.range_endpoint(
                begin_name,
                receiver,
                members,
                RangeContext {
                    dependent,
                    scope: block,
                    location,
                },
            )?;
            let receiver = self.range_use(range, named, dependent, location)?;
            let last = self.range_endpoint(
                end_name,
                receiver,
                members,
                RangeContext {
                    dependent,
                    scope: block,
                    location,
                },
            )?;
            let begin = self.range_variable(RangeVariable::Begin, auto, first, block, location)?;
            let end = self.range_variable(RangeVariable::End, auto, last, block, location)?;
            let first_binding = self.graph.declared_bindings[begin.declarator.index()]
                .ok_or(Error::Invariant("range begin binding"))?;
            let last_binding = self.graph.declared_bindings[end.declarator.index()]
                .ok_or(Error::Invariant("range end binding"))?;
            if self.graph.binding(first_binding).ty != self.graph.binding(last_binding).ty {
                return Err(Error::at(
                    location,
                    "a range's begin and end deduce different types (7.1.6.4p7)",
                ));
            }
            let left = self.range_use(Some(begin), None, dependent, location)?;
            let right = self.range_use(Some(end), None, dependent, location)?;
            let condition = self.range_binary(P::NotEqual, left, right, location)?;
            let operand = self.range_use(Some(begin), None, dependent, location)?;
            let increment = self.range_unary(P::Increment, operand, location)?;
            let operand = self.range_use(Some(begin), None, dependent, location)?;
            let element = self.range_unary(P::Multiply, operand, location)?;
            (begin, Some(end), condition, increment, element)
        };
        self.typed_expression(condition, block)?;
        self.typed_expression(increment, block)?;
        self.convert_bool(condition)?;
        self.typed_expression(element, block)?;
        self.mark_range_expression(element)?;
        let specs = self.specifiers(specifiers, block, false)?;
        let declaration = ast::InitDeclarator {
            declarator,
            initializer: Some(ast::Initializer {
                location,
                kind: ast::InitializerKind::Assign(element),
            }),
            tokens: ast::TokenSpan::default(),
        };
        self.declare_init(specs, declaration, block, location, &mut None)?;
        if let Some(range) = range
            && typed.category == V::Prvalue
            && self.graph.class_of(typed.ty).is_some()
        {
            let binding = self.graph.declared_bindings[range.declarator.index()]
                .ok_or(Error::Invariant("range object binding"))?;
            self.graph
                .range_loops
                .objects
                .insert(self.graph.binding(binding).entity);
        }
        self.graph.range_loops.records.insert(
            id,
            RangeForFacts {
                range,
                begin,
                end,
                declaration,
                condition,
                increment,
            },
        );
        Ok(())
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
    fn range_equivalents_keep_hidden_bindings_and_reuse_written_range_facts() {
        let source = SourceBuffer::new(b"int a[2];auto&range(){return a;}int f(){int __idx=0;for(const auto&x:range()){__idx+=x;}for(int x:a){__idx+=x;}return __idx;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"range-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.range_loops.records.len(), 2);
        let mut materialized = 0;
        for (&statement, facts) in &graph.range_loops.records {
            let ast::StmtKind::RangeFor {
                range, declarator, ..
            } = tree.statements.get(statement).kind
            else {
                panic!("range syntax");
            };
            assert_eq!(facts.declaration.declarator, declarator);
            let typed = graph.expression_facts.get(range).unwrap();
            assert_eq!(typed.origin, ExpressionOrigin::Implicit);
            assert!(matches!(
                graph.types.get(typed.ty).kind,
                TypeKind::Array { bound: 2, .. }
            ));
            let index_binding = graph.declared_bindings[facts.begin.declarator.index()].unwrap();
            let index = graph.binding(index_binding);
            assert_ne!(index.name, graph.entity(index.entity).name);
            let scope = graph.binding_scope(index_binding);
            assert_eq!(
                graph.own(scope, index.name.unwrap(), Filter::Ordinary),
                Some(index_binding)
            );
            if let Some(range) = facts.range {
                materialized += 1;
                let entity = graph.lower_declared(range.declarator).unwrap();
                assert!(!graph.lower_range_object(entity));
                assert!(matches!(
                    graph
                        .types
                        .get(graph.lower_entity_type(entity).unwrap())
                        .kind,
                    TypeKind::Reference {
                        kind: RefKind::Lvalue,
                        ..
                    }
                ));
            }
            assert!(
                graph
                    .expression_facts
                    .get(facts.condition)
                    .unwrap()
                    .conversion
                    .is_some()
            );
            assert!(facts.end.is_none());
        }
        assert_eq!(materialized, 1);
    }

    #[test]
    fn range_template_facts_belong_only_to_demanded_specialization_bodies() {
        let source = SourceBuffer::new(b"template<class T>int f(T(&a)[2]){for(auto&x:a){x+=1;}return a[0];}int main(){int a[2]={1,2};long b[2]={3,4};return f(a)+f(a)+f(b);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"range-template-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(graph.range_loops.records.len(), 2);
        let mut types = HashSet::new();
        for facts in graph.range_loops.records.values() {
            let entity = graph.lower_declared(facts.declaration.declarator).unwrap();
            let ty = graph.lower_entity_type(entity).unwrap();
            let TypeKind::Reference {
                element,
                kind: RefKind::Lvalue,
            } = graph.types.get(ty).kind
            else {
                panic!("deduced reference");
            };
            assert!(matches!(graph.types.get(element).kind, TypeKind::Scalar(_)));
            types.insert(element);
        }
        assert_eq!(types.len(), 2);
    }
}
