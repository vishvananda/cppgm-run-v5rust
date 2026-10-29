// Translated from sema/template_fixed_expressions.cpp. See NOTICE.
//! Definition-time facts are deliberately weaker than instantiated expression
//! facts. Names stay unknown; sizeof has a fixed type without a concrete value.
//! Query scratch dies with the walk and never publishes speculative conversions.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{BindingKind, ScopeId},
    lookup::Filter,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{LiteralForm, PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
    types::FundamentalType as F,
};

#[derive(Clone, Copy)]
struct FixedOperand {
    ty: TypeId,
    category: V,
    null_constant: bool,
    constant: bool,
}

impl FixedOperand {
    fn value(ty: TypeId, constant: bool) -> Self {
        Self {
            ty,
            category: V::Prvalue,
            null_constant: false,
            constant,
        }
    }
}

impl Analyzer<'_> {
    pub(super) fn validate_fixed_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        // The oracle avoids even the validation walk unless a literal, keyword
        // or sizeof can seed a fact. Opaque forms do not expose their operands.
        let root = id;
        self.fixed_validation_work.push((id, 0));
        while let Some((id, depth)) = self.fixed_validation_work.pop() {
            let syntax_item = *self.tree.expressions.get(id);
            let node = &syntax_item;
            if depth > 200
                || matches!(
                    node.kind,
                    ExprKind::Literal(_) | ExprKind::Keyword(_) | ExprKind::Sizeof { .. }
                )
            {
                self.fixed_validation_work.clear();
                self.fixed_expression(root, scope, 0)?;
                return Ok(());
            }
            let next = depth + 1;
            match node.kind {
                ExprKind::Paren(operand)
                | ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Member { base: operand, .. }
                | ExprKind::Cast { operand, .. }
                | ExprKind::PackExpansion(operand)
                | ExprKind::Throw(Some(operand)) => {
                    self.fixed_validation_work.push((operand, next));
                }
                ExprKind::Binary { left, right, .. }
                | ExprKind::Subscript {
                    base: left,
                    index: right,
                } => {
                    self.fixed_validation_work
                        .extend([(right, next), (left, next)]);
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    self.fixed_validation_work.extend([
                        (else_expr, next),
                        (then_expr, next),
                        (condition, next),
                    ]);
                }
                ExprKind::Call { callee, arguments } => {
                    self.fixed_validation_work.push((callee, next));
                    self.fixed_validation_work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                }
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    self.fixed_validation_work.extend(
                        self.tree
                            .expr_lists
                            .get(arguments)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                    if let Some(braced) = braced {
                        self.fixed_validation_work.push((braced, next));
                    }
                }
                ExprKind::Braced { elements, .. } => {
                    self.fixed_validation_work.extend(
                        self.tree
                            .expr_lists
                            .get(elements)
                            .iter()
                            .rev()
                            .map(|&id| (id, next)),
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn fixed_expression(
        &mut self,
        mut id: ast::ExprId,
        scope: ScopeId,
        depth: u16,
    ) -> Result<Option<FixedOperand>, Error> {
        if depth > 200 {
            return Ok(None);
        }
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        let next = depth + 1;
        match self.tree.expressions.get(id).kind {
            ExprKind::Literal(literal) => {
                let (ty, null_constant) = match literal.kind(self.literal_bytes) {
                    PostTokenKind::Literal { ty, data, form } => (
                        ty,
                        form == LiteralForm::Integer && data.as_slice().iter().all(|&b| b == 0),
                    ),
                    PostTokenKind::Character {
                        ty,
                        data,
                        suffix_begin: None,
                        ..
                    } => (ty, data.as_slice().iter().all(|&b| b == 0)),
                    _ => return Ok(None),
                };
                Ok(Some(FixedOperand {
                    ty: self.graph.types.scalar(Scalar::Basic(ty))?,
                    category: V::Prvalue,
                    null_constant,
                    constant: true,
                }))
            }
            ExprKind::Keyword(keyword @ (Keyword::True | Keyword::False | Keyword::Nullptr)) => {
                let ty = self.graph.types.scalar(if keyword == Keyword::Nullptr {
                    Scalar::Nullptr
                } else {
                    Scalar::Bool
                })?;
                Ok(Some(FixedOperand {
                    ty,
                    category: V::Prvalue,
                    null_constant: keyword != Keyword::True,
                    constant: true,
                }))
            }
            ExprKind::Sizeof { operand, .. } => {
                if let Some(operand) = operand {
                    self.fixed_expression(operand, scope, next)?;
                }
                Ok(Some(FixedOperand::value(
                    self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?,
                    true,
                )))
            }
            ExprKind::Unary { operand, .. }
            | ExprKind::Postfix { operand, .. }
            | ExprKind::Member { base: operand, .. } => {
                let operand = self.fixed_expression(operand, scope, next)?;
                self.fixed_operator(id, operand, None)
            }
            ExprKind::Binary { left, right, .. }
            | ExprKind::Subscript {
                base: left,
                index: right,
            } => {
                let left = self.fixed_expression(left, scope, next)?;
                let right = self.fixed_expression(right, scope, next)?;
                self.fixed_operator(id, left, right)
            }
            ExprKind::Call { callee, arguments } => {
                self.fixed_call(callee, arguments, id, scope, depth)
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.fixed_expression(condition, scope, next)?;
                self.fixed_expression(then_expr, scope, next)?;
                self.fixed_expression(else_expr, scope, next)?;
                Ok(None)
            }
            ExprKind::Cast { operand, .. }
            | ExprKind::PackExpansion(operand)
            | ExprKind::Throw(Some(operand)) => {
                self.fixed_expression(operand, scope, next)?;
                Ok(None)
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                if let Some(braced) = braced {
                    self.fixed_expression(braced, scope, next)?;
                }
                for syntax_index in 0..self.tree.expr_lists.get(arguments).len() {
                    let argument = self.tree.expr_lists.get(arguments)[syntax_index];
                    self.fixed_expression(argument, scope, next)?;
                }
                Ok(None)
            }
            ExprKind::Braced { elements, .. } => {
                for syntax_index in 0..self.tree.expr_lists.get(elements).len() {
                    let element = self.tree.expr_lists.get(elements)[syntax_index];
                    self.fixed_expression(element, scope, next)?;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn fixed_plain(&self, operand: FixedOperand) -> bool {
        matches!(
            self.graph.types.get(operand.ty).kind,
            TypeKind::Scalar(_) | TypeKind::Pointer(_) | TypeKind::Array { .. }
        )
    }

    fn fixed_operator(
        &mut self,
        id: ast::ExprId,
        left: Option<FixedOperand>,
        right: Option<FixedOperand>,
    ) -> Result<Option<FixedOperand>, Error> {
        let syntax_item = *self.tree.expressions.get(id);
        let node = &syntax_item;
        let failure = |message| Error::at(node.location, message);
        let kind = node.kind;
        if let ExprKind::Member { op, member, .. } = kind {
            let Some(left) = left.filter(|&operand| self.fixed_plain(operand)) else {
                return Ok(None);
            };
            let last = self
                .tree
                .name_lists
                .get(self.tree.names.get(member).components)
                .last()
                .ok_or(Error::Invariant("empty member name"))?;
            if matches!(last.kind, ast::NameKind::Destructor(_)) {
                return Ok(None);
            }
            let ty = self.decayed_type(left.ty)?;
            if op.token.as_punctuator() == Some(P::Arrow)
                && let TypeKind::Pointer(element) = self.graph.types.get(ty).kind
                && !matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Scalar(_) | TypeKind::Pointer(_)
                )
            {
                return Ok(None);
            }
            return Err(failure(
                "member access to an object of non-class type in a template definition",
            ));
        }
        let (op, binary) = match kind {
            ExprKind::Binary { op, .. } => (op.token.as_punctuator(), true),
            ExprKind::Unary { op, .. } | ExprKind::Postfix { op, .. } => {
                (op.token.as_punctuator(), false)
            }
            ExprKind::Subscript { .. } => (Some(P::LeftBracket), true),
            _ => return Err(Error::Invariant("fixed operator without operator")),
        };
        let op = op.ok_or(Error::Invariant("fixed nonpunctuator operator"))?;
        if op == P::Comma {
            return Ok(right.map(|mut right| {
                right.null_constant =
                    self.is_integral_operand(right.ty) && right.category == V::Prvalue;
                right.constant &= left.is_some_and(|left| left.constant);
                right
            }));
        }
        let Some(mut left) = left.filter(|&operand| self.fixed_plain(operand)) else {
            return Ok(None);
        };
        if matches!(
            kind,
            ExprKind::Binary {
                assignment: true,
                ..
            }
        ) {
            if !matches!(self.graph.types.get(left.ty).kind, TypeKind::Array { .. })
                && (left.category != V::Lvalue || self.type_cv(left.ty).0 & 1 != 0)
            {
                return Err(failure(
                    "assignment to a value that is not a modifiable lvalue in a template definition",
                ));
            }
            left.null_constant = false;
            left.constant = false;
            return Ok(Some(left));
        }
        if binary && !right.is_some_and(|operand| self.fixed_plain(operand)) {
            return Ok(None);
        }
        let lt = self.decayed_type(left.ty)?;
        if !binary {
            return self.fixed_unary(id, op, left, lt);
        }
        let right = right.ok_or(Error::Invariant("fixed binary without right operand"))?;
        let rt = self.decayed_type(right.ty)?;
        if op == P::LeftBracket {
            let (pointer, index) = if matches!(self.graph.types.get(lt).kind, TypeKind::Pointer(_))
            {
                (lt, rt)
            } else {
                (rt, lt)
            };
            if let TypeKind::Pointer(element) = self.graph.types.get(pointer).kind
                && self.is_integral_operand(index)
            {
                return Ok(Some(FixedOperand {
                    ty: element,
                    category: V::Lvalue,
                    null_constant: false,
                    constant: false,
                }));
            }
            return Err(failure(
                "subscript operands that are not a pointer and an integer in a template definition",
            ));
        }
        self.fixed_binary(id, op, left, right, lt, rt)
    }

    fn fixed_unary(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: FixedOperand,
        lt: TypeId,
    ) -> Result<Option<FixedOperand>, Error> {
        let error = |message| Error::at(self.tree.expressions.get(id).location, message);
        let result = match op {
            P::Multiply => {
                let TypeKind::Pointer(element) = self.graph.types.get(lt).kind else {
                    return Err(error(
                        "indirection through an operand that is not a pointer in a template definition",
                    ));
                };
                FixedOperand {
                    ty: element,
                    category: V::Lvalue,
                    null_constant: false,
                    constant: false,
                }
            }
            P::BitAnd => {
                if left.category != V::Lvalue {
                    return Err(error(
                        "the address of a value that is not an lvalue in a template definition",
                    ));
                }
                FixedOperand::value(self.graph.types.pointer(left.ty)?, false)
            }
            P::Increment | P::Decrement => {
                if left.category != V::Lvalue
                    || self.type_cv(left.ty).0 & 1 != 0
                    || matches!(self.graph.types.get(left.ty).kind, TypeKind::Array { .. })
                    || !(self.is_arithmetic_type(lt)
                        || matches!(self.graph.types.get(lt).kind, TypeKind::Pointer(_)))
                {
                    return Err(error(
                        "increment of an operand that is not a modifiable arithmetic or pointer lvalue in a template definition",
                    ));
                }
                if matches!(self.tree.expressions.get(id).kind, ExprKind::Postfix { .. }) {
                    FixedOperand::value(lt, false)
                } else {
                    FixedOperand {
                        constant: false,
                        null_constant: false,
                        ..left
                    }
                }
            }
            P::LogicalNot => {
                if matches!(
                    self.graph.types.get(lt).kind,
                    TypeKind::Scalar(Scalar::Void)
                ) {
                    return Err(error(
                        "logical negation of a void operand in a template definition",
                    ));
                }
                FixedOperand::value(self.graph.types.scalar(Scalar::Bool)?, left.constant)
            }
            P::Add | P::Subtract | P::Complement => {
                if op == P::Add && matches!(self.graph.types.get(lt).kind, TypeKind::Pointer(_)) {
                    FixedOperand::value(lt, false)
                } else {
                    if if op == P::Complement {
                        !self.is_integral_operand(lt)
                    } else {
                        !self.is_arithmetic_type(lt)
                    } {
                        return Err(error(
                            "an operand of a unary arithmetic operator that it does not take in a template definition",
                        ));
                    }
                    FixedOperand::value(self.promoted_type(lt)?, left.constant)
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(FixedOperand {
            null_constant: result.constant && self.is_integral_operand(result.ty),
            ..result
        }))
    }

    fn fixed_binary(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: FixedOperand,
        right: FixedOperand,
        lt: TypeId,
        rt: TypeId,
    ) -> Result<Option<FixedOperand>, Error> {
        let lp = matches!(self.graph.types.get(lt).kind, TypeKind::Pointer(_));
        let rp = matches!(self.graph.types.get(rt).kind, TypeKind::Pointer(_));
        let ln = matches!(
            self.graph.types.get(lt).kind,
            TypeKind::Scalar(Scalar::Nullptr)
        );
        let rn = matches!(
            self.graph.types.get(rt).kind,
            TypeKind::Scalar(Scalar::Nullptr)
        );
        let mut valid = ![lt, rt].iter().any(|&ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        });
        let arithmetic = self.is_arithmetic_type(lt) && self.is_arithmetic_type(rt);
        let integral = self.is_integral_operand(lt) && self.is_integral_operand(rt);
        let result = match op {
            P::Multiply | P::Divide => {
                valid &= arithmetic;
                if valid {
                    Some(self.arithmetic_type(lt, rt)?)
                } else {
                    None
                }
            }
            P::Remainder | P::BitAnd | P::BitOr | P::Xor => {
                valid &= integral;
                if valid {
                    Some(self.arithmetic_type(lt, rt)?)
                } else {
                    None
                }
            }
            P::ShiftLeft | P::ShiftRight => {
                valid &= integral;
                if valid {
                    Some(self.promoted_type(lt)?)
                } else {
                    None
                }
            }
            P::Add | P::Subtract => {
                if arithmetic {
                    Some(self.arithmetic_type(lt, rt)?)
                } else if lp && self.is_integral_operand(rt) {
                    Some(lt)
                } else if op == P::Add && rp && self.is_integral_operand(lt) {
                    Some(rt)
                } else if op == P::Subtract && lp && rp {
                    Some(self.graph.types.scalar(Scalar::Basic(F::LongInt))?)
                } else {
                    valid = false;
                    None
                }
            }
            P::Less | P::Greater | P::LessEqual | P::GreaterEqual | P::Equal | P::NotEqual => {
                valid &= arithmetic
                    || ((lp || rp || ln || rn)
                        && (lp || left.null_constant || ln)
                        && (rp || right.null_constant || rn));
                Some(self.graph.types.scalar(Scalar::Bool)?)
            }
            P::LogicalAnd | P::LogicalOr => Some(self.graph.types.scalar(Scalar::Bool)?),
            _ => return Ok(None),
        };
        if !valid {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "operands a built-in binary operator does not take in a template definition",
            ));
        }
        Ok(result.map(|ty| {
            let constant = left.constant && right.constant && !(lp || rp);
            FixedOperand {
                ty,
                category: V::Prvalue,
                null_constant: constant && self.is_integral_operand(ty),
                constant,
            }
        }))
    }

    fn fixed_without_associations(&self, mut ty: TypeId) -> bool {
        for _ in 0..=16 {
            match self.graph.types.get(ty).kind {
                TypeKind::Scalar(_) => return true,
                TypeKind::Pointer(element) | TypeKind::Array { element, .. } => ty = element,
                _ => return false,
            }
        }
        false
    }

    fn fixed_call(
        &mut self,
        callee: ast::ExprId,
        arguments: List<ast::ExprId>,
        id: ast::ExprId,
        scope: ScopeId,
        depth: u16,
    ) -> Result<Option<FixedOperand>, Error> {
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let syntax = argument_buffer.as_slice();
        let mut arguments = [None; 16];
        let mut known = syntax.len() <= 16;
        let mut constant = false;
        for (index, &argument) in syntax.iter().enumerate() {
            let facts = self.fixed_expression(argument, scope, depth + 1)?;
            known &= facts.is_some_and(|facts| {
                self.fixed_plain(facts) && self.fixed_without_associations(facts.ty)
            });
            constant |= facts.is_some_and(|facts| facts.constant);
            if let Some(slot) = arguments.get_mut(index) {
                *slot = facts;
            }
        }
        let ExprKind::Id(name) = self.tree.expressions.get(callee).kind else {
            self.fixed_expression(callee, scope, depth + 1)?;
            return Ok(None);
        };
        if !known || !constant {
            return Ok(None);
        }
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if node.global || components.len() != 1 || components[0].arguments.is_some() {
            return Ok(None);
        }
        let ast::NameKind::Identifier(key) = components[0].kind else {
            return Ok(None);
        };
        let found = self.unqualified_result(scope, key.into(), Filter::Ordinary)?;
        if found.ambiguous
            || !found
                .binding
                .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
            || found.functions.as_slice().is_empty()
        {
            return Ok(None);
        }
        let mut viable = 0;
        let mut result = None;
        for &binding in found.functions.as_slice() {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let Some(ty) = entity.ty else {
                return Ok(None);
            };
            if entity.context().template_head.is_some()
                || entity.callable().member_function.is_some()
                || self.exception_dependent_type(ty)
                || entity.callable().builtin.is_some()
            {
                return Ok(None);
            }
            let (target, count, variadic) = match &self.graph.types.get(ty).kind {
                TypeKind::Function {
                    result,
                    parameters,
                    variadic,
                    ..
                } => (*result, parameters.len(), *variadic),
                _ => return Ok(None),
            };
            if syntax.len() > count && !variadic {
                continue;
            }
            let entity = self.graph.binding(binding).entity;
            let mut accepted = true;
            for index in 0..count {
                let Some(argument) = arguments.get(index).filter(|_| index < syntax.len()) else {
                    accepted = self.graph.function_default(entity, index).is_some();
                    if !accepted {
                        break;
                    }
                    continue;
                };
                let argument = argument.ok_or(Error::Invariant("unknown fixed call argument"))?;
                let parameter = self.graph.types.function_parameter(ty, index)?;
                let object = match self.graph.types.get(parameter).kind {
                    TypeKind::Reference { element, .. } => element,
                    _ => parameter,
                };
                if !matches!(
                    self.graph.types.get(object).kind,
                    TypeKind::Scalar(_) | TypeKind::Pointer(_)
                ) {
                    continue;
                }
                let mut operand = ExprFacts::new(argument.ty, argument.category);
                operand.set_null_literal(argument.null_constant);
                if self.scalar_sequence(operand, parameter, false)?.is_none() {
                    accepted = false;
                    break;
                }
            }
            if accepted {
                viable += 1;
                result = Some(target);
            }
        }
        if viable == 0 {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "no function the nondependent call can call in a template definition",
            ));
        }
        if viable != 1 {
            return Ok(None);
        }
        let ty = result.ok_or(Error::Invariant("fixed viable call without result"))?;
        Ok(Some(match self.graph.types.get(ty).kind {
            TypeKind::Reference { element, kind } => FixedOperand {
                ty: element,
                category: if kind == RefKind::Lvalue
                    || matches!(
                        self.graph.types.get(element).kind,
                        TypeKind::Function { .. }
                    ) {
                    V::Lvalue
                } else {
                    V::Xvalue
                },
                null_constant: false,
                constant: false,
            },
            _ => FixedOperand::value(ty, false),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{analyzer::Analyzer, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn definition_validation_keeps_concrete_facts_and_demand_unpublished() {
        let source = SourceBuffer::new(
            b"int take(const int&);template<class T> int unused(T value){\
                const int local=take(sizeof(T));\
                switch(value){case sizeof(T):break;default:break;}\
                return take(sizeof(T))+local;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"template-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let take = cursor.intern_name(b"take").unwrap();
        let expression_count = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let template = tree.decl_lists.get(declarations)[1];
        let ast::DeclKind::Template { declaration, .. } = tree.declarations.get(template).kind
        else {
            unreachable!()
        };
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(declaration).kind else {
            unreachable!()
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            unreachable!()
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            unreachable!()
        };
        let ast::StmtKind::Return(Some(result)) = tree
            .statements
            .get(*tree.stmt_lists.get(statements).last().unwrap())
            .kind
        else {
            unreachable!()
        };
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        // Fixed calls use lightweight operands, not the runtime fact table.
        assert!(graph.expression_facts.get(result).is_err());
        assert!(graph.root_demands.as_slice().is_empty());
        let take = graph.own(graph.global, take, Filter::Ordinary).unwrap();
        assert!(
            graph
                .entity(graph.binding(take).entity)
                .callable()
                .demands
                .as_slice()
                .is_empty()
        );
        assert_eq!(tree.expressions.mark(), expression_count);
        assert_eq!(tree.declarations.mark(), declaration_count);
    }
}
