// Translated from sema/template_placeholders.cpp::decltype_words/size_words. See NOTICE.
//! Structural keys borrow parsed operands and retain canonical semantic identities.
//! A key is local to one lookup environment; expression facts are never its cache.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{EntityId, ScopeId, ScopeKind},
    integral::IntegralKind,
    types::{Scalar, TypeId},
};
use crate::{
    lex::{
        PostTokenKind,
        identifiers::NameId,
        vocabulary::{Keyword, SimpleToken},
    },
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyWord {
    Node(KeyNode),
    Transform(crate::lex::type_transforms::TypeTransform),
    Predicate(crate::lex::predicates::TypePredicate),
    Count(usize),
    Flag(bool),
    Operator(SimpleToken),
    Keyword(Option<Keyword>),
    Type(TypeId),
    Entity(EntityId),
    Parameter {
        index: usize,
        level: u32,
    },
    Identifier(NameId),
    FreeName(NameId),
    DependentCallee(EntityId, NameId),
    ResolvedFunction(EntityId),
    Constant(TypeId, u64),
    FloatingLiteral {
        spelling: Vec<u8>,
        ty: TypeId,
        bits: u64,
    },
    Spelling(Vec<u8>),
    Absent,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyNode {
    Id,
    Name,
    Arguments,
    OffsetOf,
    OffsetOfBase,
    Typeof,
    VaArg,
    BuiltinValue(ast::ValueBuiltin),
    ComplexPart,
    Keyword,
    This,
    Paren,
    Unary,
    Postfix,
    Binary,
    Assign,
    Conditional,
    Call,
    Subscript,
    Member,
    Braced,
    FunctionalCast,
    Cast,
    Sizeof,
    Trait,
    New,
    Delete,
    Throw,
    PackExpansion,
    Fold,
    SizeofPack,
    IntegerPack,
    BitInt,
    Function,
}
#[derive(Default)]
pub(super) struct ExpressionKey {
    pub words: Vec<KeyWord>,
    pub dependent: bool,
    pub precise: bool,
}
impl Analyzer<'_> {
    pub(super) fn expression_key(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<Vec<KeyWord>>, Error> {
        self.operand_key(operand, scope)
            .map(|key| key.map(|key| key.words))
    }

    pub(super) fn operand_key(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<ExpressionKey>, Error> {
        let mut key = ExpressionKey::default();
        self.key_expression(operand, scope, &mut key)
            .map(|known| known.then_some(key))
    }

    pub(super) fn fixed_operand_key(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<ExpressionKey>, Error> {
        let mut key = ExpressionKey {
            precise: true,
            ..ExpressionKey::default()
        };
        self.key_expression(operand, scope, &mut key)
            .map(|known| known.then_some(key))
    }

    pub(super) fn key_type(&mut self, ty: TypeId, key: &mut ExpressionKey) -> Result<(), Error> {
        key.dependent |= self.exception_dependent_type(ty);
        key.words
            .push(KeyWord::Type(self.graph.types.canonical_function(ty)?));
        Ok(())
    }

    fn key_fold(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let ExprKind::Fold {
            op,
            pattern,
            init,
            right,
        } = self.tree.expressions.get(operand).kind
        else {
            return Err(Error::Invariant("fold key without fold syntax"));
        };
        key.words.extend([
            KeyWord::Node(KeyNode::Fold),
            KeyWord::Operator(op.token),
            KeyWord::Flag(right),
            KeyWord::Flag(init.is_some()),
        ]);
        Ok(self.key_expression(pattern, scope, key)?
            && match init {
                Some(init) => self.key_expression(init, scope, key)?,
                None => true,
            })
    }
    #[expect(
        clippy::too_many_lines,
        reason = "One structural-key dispatch over the original syntax vocabulary"
    )]
    fn key_expression(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let operand_id = operand;
        let expr = *self.tree.expressions.get(operand);
        self.check_stack().map_err(|error| match error {
            Error::Limit("constructs nested too deeply") => {
                Error::at(expr.location, "constructs nested too deeply")
            }
            other => other,
        })?;
        match expr.kind {
            ExprKind::Id(name) => {
                return self.key_name(name, scope, true, None, key);
            }
            ExprKind::ComplexPart { imaginary, operand } => {
                key.words.push(KeyWord::Node(KeyNode::ComplexPart));
                key.words.push(KeyWord::Flag(imaginary));
                return self.key_expression(operand, scope, key);
            }
            ExprKind::Literal(literal) => self.key_literal(literal, key)?,
            ExprKind::Keyword(keyword) => self.key_keyword(keyword, scope, key)?,
            ExprKind::Fold { .. } => return self.key_fold(operand, scope, key),
            ExprKind::Paren(inner) | ExprKind::PackExpansion(inner) => {
                key.words
                    .push(KeyWord::Node(if matches!(expr.kind, ExprKind::Paren(_)) {
                        KeyNode::Paren
                    } else {
                        KeyNode::PackExpansion
                    }));
                return self.key_expression(inner, scope, key);
            }
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                key.words.push(KeyWord::Node(
                    if matches!(expr.kind, ExprKind::Unary { .. }) {
                        KeyNode::Unary
                    } else {
                        KeyNode::Postfix
                    },
                ));
                key.words.push(KeyWord::Operator(op.token));
                return self.key_expression(operand, scope, key);
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment,
            } => {
                key.words.push(KeyWord::Node(if assignment {
                    KeyNode::Assign
                } else {
                    KeyNode::Binary
                }));
                key.words.push(KeyWord::Operator(op.token));
                return Ok(self.key_expression(left, scope, key)?
                    && self.key_expression(right, scope, key)?);
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                key.words.push(KeyWord::Node(KeyNode::Conditional));
                return Ok(self.key_expression(condition, scope, key)?
                    && self.key_expression(then_expr, scope, key)?
                    && self.key_expression(else_expr, scope, key)?);
            }
            ExprKind::BitInt(operand) => {
                key.words.push(KeyWord::Node(KeyNode::BitInt));
                return self.key_expression(operand, scope, key);
            }
            ExprKind::IntegerPack(operand) => {
                key.words.push(KeyWord::Node(KeyNode::IntegerPack));
                return self.key_expression(operand, scope, key);
            }
            ExprKind::Call { callee, arguments } => {
                key.words.extend([
                    KeyWord::Node(KeyNode::Call),
                    KeyWord::Count(self.tree.expr_lists.get(arguments).len()),
                ]);
                let known = if let ExprKind::Id(name) = self.tree.expressions.get(callee).kind
                    && !self.tree.names.get(name).global
                    && self
                        .tree
                        .name_lists
                        .get(self.tree.names.get(name).components)
                        .len()
                        == 1
                {
                    self.key_name(name, scope, true, Some(arguments), key)?
                } else {
                    self.key_expression(callee, scope, key)?
                };
                if !known {
                    return Ok(false);
                }
                return self.key_arguments(arguments, scope, key);
            }
            ExprKind::Subscript { base, index } => {
                key.words.push(KeyWord::Node(KeyNode::Subscript));
                return Ok(self.key_expression(base, scope, key)?
                    && self.key_expression(index, scope, key)?);
            }
            ExprKind::Member { base, op, member } => {
                key.words
                    .extend([KeyWord::Node(KeyNode::Member), KeyWord::Operator(op.token)]);
                return Ok(self.key_expression(base, scope, key)?
                    && self.key_name(member, scope, false, None, key)?);
            }
            ExprKind::Braced { elements, .. } => {
                key.words.extend([
                    KeyWord::Node(KeyNode::Braced),
                    KeyWord::Count(self.tree.expr_lists.get(elements).len()),
                ]);
                return self.key_arguments(elements, scope, key);
            }
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                key.words
                    .extend([KeyWord::Node(KeyNode::Cast), KeyWord::Keyword(keyword)]);
                let ty = self.type_syntax(ty, scope)?;
                self.key_type(ty, key)?;
                return self.key_expression(operand, scope, key);
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                key.words.extend([
                    KeyWord::Node(KeyNode::FunctionalCast),
                    KeyWord::Count(self.tree.expr_lists.get(arguments).len()),
                    KeyWord::Flag(braced.is_some()),
                ]);
                let ty = self.specifiers(specifiers, scope, false)?.ty;
                self.key_type(ty, key)?;
                return if let Some(operand) = braced {
                    self.key_expression(operand, scope, key)
                } else {
                    self.key_arguments(arguments, scope, key)
                };
            }
            ExprKind::Sizeof { ty, operand, .. } | ExprKind::TypeTrait { ty, operand, .. } => {
                return self.key_size(operand, ty, scope, operand_id, expr, key);
            }
            ExprKind::Typeof { ty, operand } => return self.key_typeof(ty, operand, scope, key),
            ExprKind::TypeTransform { transform, types } => {
                key.words.extend([
                    KeyWord::Transform(transform),
                    KeyWord::Count(self.tree.type_lists.get(types).len()),
                ]);
                for index in 0..self.tree.type_lists.get(types).len() {
                    let ty = self.type_syntax(self.tree.type_lists.get(types)[index], scope)?;
                    self.key_type(ty, key)?;
                }
                return Ok(true);
            }
            ExprKind::TypePredicate { predicate, types } => {
                key.words.extend([
                    KeyWord::Predicate(predicate),
                    KeyWord::Count(self.tree.type_lists.get(types).len()),
                ]);
                for index in 0..self.tree.type_lists.get(types).len() {
                    let ty = self.type_syntax(self.tree.type_lists.get(types)[index], scope)?;
                    self.key_type(ty, key)?;
                }
                return Ok(true);
            }
            ExprKind::OffsetOfBase(ty) => {
                key.words.push(KeyWord::Node(KeyNode::OffsetOfBase));
                let ty = self.type_syntax(ty, scope)?;
                self.key_type(ty, key)?;
                return Ok(true);
            }
            ExprKind::OffsetOf { ty, designator } => {
                key.words.push(KeyWord::Node(KeyNode::OffsetOf));
                let ty = self.type_syntax(ty, scope)?;
                self.key_type(ty, key)?;
                return self.key_expression(designator, scope, key);
            }
            ExprKind::BuiltinValue {
                builtin,
                operand,
                ty,
            } => {
                key.words
                    .push(KeyWord::Node(KeyNode::BuiltinValue(builtin)));
                let ty = self.type_syntax(ty, scope)?;
                self.key_type(ty, key)?;
                return self.key_expression(operand, scope, key);
            }
            ExprKind::VaArg { operand, ty } => {
                key.words.push(KeyWord::Node(KeyNode::VaArg));
                let ty = self.type_syntax(ty, scope)?;
                self.key_type(ty, key)?;
                return self.key_expression(operand, scope, key);
            }
            ExprKind::New { .. } => return self.key_allocation(expr.kind, scope, key),
            ExprKind::Delete {
                global,
                array,
                operand,
            } => {
                key.words.extend([
                    KeyWord::Node(KeyNode::Delete),
                    KeyWord::Flag(global),
                    KeyWord::Flag(array),
                ]);
                return self.key_expression(operand, scope, key);
            }
            ExprKind::Throw(operand) => {
                key.words.extend([
                    KeyWord::Node(KeyNode::Throw),
                    KeyWord::Flag(operand.is_none()),
                ]);
                return if let Some(operand) = operand {
                    self.key_expression(operand, scope, key)
                } else {
                    Ok(true)
                };
            }
            // C++ leaves lambda and statement-block operands unkeyed. Packs require their
            // declaration owner, which is outside the PA6 template boundary.
            ExprKind::Lambda(_) | ExprKind::Statement(_) => return Ok(false),
            ExprKind::SizeofPack(name) => {
                let Some(binding) =
                    self.unqualified(scope, name, super::lookup::Filter::Ordinary)?
                else {
                    return Ok(false);
                };
                let id = binding;
                let binding = self.graph.binding(id);
                // C++ bind_pack gives a whole-pack frame alias no entity.
                // Its sizeof... is evaluable, but is not the declaration's
                // positional pack key. Rust retains the parameter identity
                // on that alias, so distinguish the typed binding here.
                if binding.ty.is_some_and(|ty| {
                    matches!(
                        self.graph.types.get(ty).kind,
                        super::types::TypeKind::BoundPack { .. }
                    )
                }) {
                    return Ok(false);
                }
                self.graph.analysis.keyed_sizeof_packs = true;
                if binding.kind == super::graph::BindingKind::Parameter
                    && binding.ty.is_some()
                    && self.graph.prototype_parameter(binding.entity).is_some()
                {
                    key.words.push(KeyWord::Node(KeyNode::SizeofPack));
                    key.words.push(KeyWord::Flag(true));
                    self.key_binding(id, scope, key)?;
                    key.dependent = true;
                    return Ok(true);
                }
                if self
                    .graph
                    .entity(binding.entity)
                    .context()
                    .template_parameter
                    .is_none()
                {
                    return Ok(false);
                }
                let Some(ty) = self.graph.entity(binding.entity).ty else {
                    return Ok(false);
                };
                key.words.push(KeyWord::Node(KeyNode::SizeofPack));
                self.key_type(ty, key)?;
                key.dependent = true;
            }
        }
        Ok(true)
    }

    fn key_arguments(
        &mut self,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        for syntax_index in 0..self.tree.expr_lists.get(arguments).len() {
            let argument = self.tree.expr_lists.get(arguments)[syntax_index];
            if !self.key_expression(argument, scope, key)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn key_keyword(
        &mut self,
        keyword: Keyword,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<(), Error> {
        if matches!(keyword, Keyword::True | Keyword::False) {
            key.words.push(KeyWord::Constant(
                self.graph.types.scalar(Scalar::Bool)?,
                u64::from(keyword == Keyword::True),
            ));
        } else {
            key.words.extend([
                KeyWord::Node(KeyNode::Keyword),
                KeyWord::Keyword(Some(keyword)),
            ]);
            if keyword == Keyword::This {
                self.key_this(scope, key)?;
            }
        }
        Ok(())
    }

    fn key_typeof(
        &mut self,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        key.words.push(KeyWord::Node(KeyNode::Typeof));
        if let Some(ty) = ty {
            let ty = self.type_syntax(ty, scope)?;
            self.key_type(ty, key)?;
        }
        if let Some(operand) = operand {
            self.key_expression(operand, scope, key)
        } else {
            Ok(true)
        }
    }

    fn key_this(&mut self, scope: ScopeId, key: &mut ExpressionKey) -> Result<(), Error> {
        let mut current = Some(scope);
        key.words.push(KeyWord::Node(KeyNode::This));
        if let Some(ty) = self.current_object_type()? {
            let pointer = self.graph.types.pointer(ty)?;
            let pointer = self.graph.types.canonical_function(pointer)?;
            key.words.push(KeyWord::Type(pointer));
            key.dependent |= self.graph.dependent_type(ty);
            return Ok(());
        }
        while let Some(scope) = current {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::Class {
                if let Some(entity) = node.owner {
                    key.words.push(KeyWord::Entity(entity));
                    key.dependent |= self.graph.entity(entity).context().template_head.is_some();
                } else {
                    key.words.push(KeyWord::Absent);
                }
                return Ok(());
            }
            current = node.parent;
        }
        key.words.push(KeyWord::Absent);
        Ok(())
    }

    fn key_literal(&mut self, literal: ast::Literal, key: &mut ExpressionKey) -> Result<(), Error> {
        let typed = match literal.kind(self.literal_bytes) {
            PostTokenKind::Literal { ty, data, .. }
            | PostTokenKind::Character {
                ty,
                data,
                multicharacter: false,
                suffix_begin: None,
                ..
            } => Some((ty, data)),
            _ => None,
        };
        if let Some((ty, data)) = typed
            && IntegralKind::of(Scalar::Basic(ty)).is_some()
        {
            let mut bytes = [0u8; 8];
            let count = data.as_slice().len().min(8);
            bytes[..count].copy_from_slice(&data.as_slice()[..count]);
            key.words.push(KeyWord::Constant(
                self.graph.types.scalar(Scalar::Basic(ty))?,
                u64::from_le_bytes(bytes),
            ));
        } else if let Some((
            ty @ (crate::types::FundamentalType::Float | crate::types::FundamentalType::Double),
            data,
        )) = typed
        {
            let mut bytes = [0u8; 8];
            let count = data.as_slice().len().min(8);
            bytes[..count].copy_from_slice(&data.as_slice()[..count]);
            key.words.push(KeyWord::FloatingLiteral {
                spelling: self
                    .literal_bytes
                    .get(literal.spelling(self.literal_bytes))
                    .to_vec(),
                ty: self.graph.types.scalar(Scalar::Basic(ty))?,
                bits: u64::from_le_bytes(bytes),
            });
        } else {
            key.words.push(KeyWord::Spelling(
                self.literal_bytes
                    .get(literal.spelling(self.literal_bytes))
                    .to_vec(),
            ));
            // For nonintegral literals C++ includes spelling even when IEEE
            // bits are known. Identical spelling already determines those bits.
        }
        Ok(())
    }

    fn key_size(
        &mut self,
        operand: Option<ast::ExprId>,
        ty: Option<ast::TypeId>,
        scope: ScopeId,
        id: ast::ExprId,
        expr: ast::Expression,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let (node, keyword) = if let ExprKind::TypeTrait { keyword, .. } = expr.kind {
            (KeyNode::Trait, Some(keyword))
        } else {
            (KeyNode::Sizeof, None)
        };
        if let Some(ty) = ty {
            key.words.extend([
                KeyWord::Node(node),
                KeyWord::Keyword(keyword),
                KeyWord::Flag(true),
            ]);
            let ty = self.type_syntax(ty, scope)?;
            self.key_type(ty, key)?;
            return Ok(true);
        }
        if let Some(operand) = operand
            && !self.instantiation_dependent_operand(operand, scope)?
            && let Some(value) = self.constant_value_for_key(id, scope)?
        {
            key.words.push(KeyWord::Constant(value.ty, value.bits));
            return Ok(true);
        }
        key.words.extend([
            KeyWord::Node(node),
            KeyWord::Keyword(keyword),
            KeyWord::Flag(false),
        ]);
        if let Some(operand) = operand {
            self.key_expression(operand, scope, key)
        } else {
            key.words.push(KeyWord::Absent);
            Ok(true)
        }
    }

    fn constant_value_for_key(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<super::constants::Constant>, Error> {
        // Prototype Binding.ty retains the source parameter type until
        // expression typing adjusts its object type.
        let saved = self.evaluating_noexcept;
        self.evaluating_noexcept = false;
        let result = self.constant_value(id, scope, false);
        self.evaluating_noexcept = saved;
        result
    }

    fn key_allocation(
        &mut self,
        kind: ExprKind,
        scope: ScopeId,
        key: &mut ExpressionKey,
    ) -> Result<bool, Error> {
        let ExprKind::New {
            global,
            placement,
            ty,
            initializer,
            ..
        } = kind
        else {
            return Err(Error::Invariant("allocation key without new"));
        };
        let placement = placement.unwrap_or_default();
        let (form, arguments) = match initializer {
            None => (0, List::default()),
            Some(ast::Initializer {
                kind: ast::InitializerKind::Parenthesized(arguments),
                ..
            }) => (1, arguments),
            Some(ast::Initializer {
                kind: ast::InitializerKind::Braced(expression),
                ..
            }) => {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expression).kind
                else {
                    return Ok(false);
                };
                (2, elements)
            }
            _ => return Ok(false),
        };
        let count = self.tree.expr_lists.get(arguments).len();
        let placed = self.tree.expr_lists.get(placement).len();
        if placed > 0xffff || count > 0xffff {
            return Ok(false);
        }
        let Some((ty, bound)) = self.allocation_key_type(ty, scope)? else {
            return Ok(false);
        };
        key.words.extend([
            KeyWord::Node(KeyNode::New),
            KeyWord::Count(placed),
            KeyWord::Flag(global),
            KeyWord::Flag(bound.is_some()),
            KeyWord::Count(form),
            KeyWord::Count(count),
        ]);
        self.key_type(ty, key)?;
        if let Some(bound) = bound
            && !self.key_expression(bound, scope, key)?
        {
            return Ok(false);
        }
        Ok(self.key_arguments(placement, scope, key)?
            && self.key_arguments(arguments, scope, key)?)
    }
}
