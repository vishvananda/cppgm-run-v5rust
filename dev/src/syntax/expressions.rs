// Translated from syntax/parser_expressions.cpp. See NOTICE.
use super::ast::SpecifierKind;
use super::{
    ParseError,
    ast::{ExprId, ExprKind, Expression, Operator, Specifier, TypeId},
    cursor::TokenKind,
    parser::Parser,
    storage::List,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
use crate::support::source::SourceOffset;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Coroutine {
    Await,
    Yield,
    Return,
}

enum Operand {
    Type(TypeId),
    Expression(ExprId),
    Compound(TypeId),
}
impl Parser<'_> {
    // Hosted contextual operators remain ordinary identifiers when declared.
    // The frozen parser represents an operator as a call of an unspellable
    // name, retained by template bodies until semantic demand.
    pub(super) fn coroutine_keyword(&mut self) -> Result<Option<Coroutine>, ParseError> {
        if self.environment != super::parser::Environment::Hosted {
            return Ok(None);
        }
        let TokenKind::Identifier(name) = self.token()?.kind else {
            return Ok(None);
        };
        let operation = match self.cursor.name(name) {
            b"co_await" => Coroutine::Await,
            b"co_yield" => Coroutine::Yield,
            b"co_return" => Coroutine::Return,
            _ => return Ok(None),
        };
        if self.scopes.lookup_name(name, false).is_some()
            || matches!(self.cursor.peek(1)?.kind, TokenKind::Simple(op)
                if matches!(op.token.as_punctuator(), Some(P::RightParen | P::Comma | P::Assign | P::RightBracket)))
            || (operation != Coroutine::Return && self.at_ahead(P::Semicolon, 1)?)
        {
            return Ok(None);
        }
        Ok(Some(operation))
    }

    pub(super) fn coroutine_operation(
        &mut self,
        operation: Coroutine,
        operand: Option<ExprId>,
        location: SourceOffset,
    ) -> Result<ExprId, ParseError> {
        use super::ast::{NameComponent, NameKind, PrefixState, QualifiedName, TemplatePrefix};
        let spelling: &[u8] = match operation {
            Coroutine::Await => b" co_await",
            Coroutine::Yield => b" co_yield",
            Coroutine::Return => b" co_return",
        };
        let name = self.cursor.intern_name(spelling)?;
        let start = self.tree.name_lists.begin();
        self.tree.name_lists.push(NameComponent {
            location,
            kind: NameKind::Identifier(name),
            template_keyword: false,
            expanded: false,
            arguments: None,
        });
        let components = self.tree.name_lists.finish(start)?;
        let name = self.tree.names.add(QualifiedName {
            location,
            global: false,
            typename_keyword: false,
            components,
            prefix_scope: None,
            prefix_state: PrefixState::Unqualified,
            prefix_template: TemplatePrefix::Ordinary,
            prefix_guesses: super::scopes::Guesses::default(),
            mentions_parameters: false,
            binding: None,
        })?;
        let callee = self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Id(name),
        })?;
        let start = self.tree.expr_lists.begin();
        if let Some(operand) = operand {
            self.tree.expr_lists.push(operand);
        }
        let arguments = self.tree.expr_lists.finish(start)?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Call { callee, arguments },
        })
    }

    pub(super) fn continue_assignment_expression(
        &mut self,
        primary: ExprId,
    ) -> Result<ExprId, ParseError> {
        let postfix = self.postfix(primary)?;
        let binary = self.binary_rest(postfix, 3)?;
        self.assignment_rest(binary)
    }
    fn continue_expression(&mut self, primary: ExprId) -> Result<ExprId, ParseError> {
        let assignment = self.continue_assignment_expression(primary)?;
        self.expression_rest(assignment)
    }
    pub(super) fn expression(&mut self) -> Result<ExprId, ParseError> {
        let left = self.assignment_expression()?;
        self.expression_rest(left)
    }
    pub(super) fn expression_rest(&mut self, mut left: ExprId) -> Result<ExprId, ParseError> {
        while self.at(P::Comma)? && !self.at_ahead(P::Ellipsis, 1)? {
            self.cursor.advance()?;
            let right = self.assignment_expression()?;
            let location = self.tree.expressions.get(left).location;
            left = self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Binary {
                    op: Self::make_operator(P::Comma)?,
                    left,
                    right,
                    assignment: false,
                },
            })?;
        }
        Ok(left)
    }
    pub(super) fn initializer_clause(&mut self) -> Result<ExprId, ParseError> {
        if self.at(P::LeftBrace)? {
            self.braced_list()
        } else {
            self.assignment_expression()
        }
    }
    pub(super) fn assignment_expression(&mut self) -> Result<ExprId, ParseError> {
        // Assignment and throw recurse without adding a C++ nesting level.
        // Check their physical frame budget before another recursive call.
        self.stack_check()?;
        if self.keyword()? == Some(K::Throw) {
            let location = self.location()?;
            self.cursor.advance()?;
            let operand = if self.operand_start()? {
                Some(self.assignment_expression()?)
            } else {
                None
            };
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Throw(operand),
            });
        }
        if self.coroutine_keyword()? == Some(Coroutine::Yield) {
            let location = self.location()?;
            self.cursor.advance()?;
            let operand = self.initializer_clause()?;
            return self.coroutine_operation(Coroutine::Yield, Some(operand), location);
        }
        let left = self.binary(3)?;
        self.assignment_rest(left)
    }
    pub(super) fn assignment_rest(&mut self, left: ExprId) -> Result<ExprId, ParseError> {
        let location = self.tree.expressions.get(left).location;
        if self.accept(P::Question)? {
            let then_expr = self.expression()?;
            self.expect(P::Colon)?;
            let else_expr = self.assignment_expression()?;
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Conditional {
                    condition: left,
                    then_expr,
                    else_expr,
                },
            });
        }
        if matches!(
            self.punctuator()?,
            Some(
                P::Assign
                    | P::AddAssign
                    | P::SubtractAssign
                    | P::MultiplyAssign
                    | P::DivideAssign
                    | P::RemainderAssign
                    | P::ShiftLeftAssign
                    | P::ShiftRightAssign
                    | P::AndAssign
                    | P::XorAssign
                    | P::OrAssign
            )
        ) && !self.at_ahead(P::Ellipsis, 1)?
        {
            let op = self.operator()?;
            let right = self.initializer_clause()?;
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Binary {
                    op,
                    left,
                    right,
                    assignment: true,
                },
            });
        }
        Ok(left)
    }
    fn binary_operator(&mut self) -> Result<Option<(Operator, u8, usize)>, ParseError> {
        let TokenKind::Simple(mut op) = self.token()?.kind else {
            return Ok(None);
        };
        if self.angle && op.token.as_punctuator() == Some(P::Greater) {
            return Ok(None);
        }
        let mut width = 1;
        let precedence = match op.token.as_punctuator() {
            Some(P::DotStar | P::ArrowStar) => 13,
            Some(P::Multiply | P::Divide | P::Remainder) => 12,
            Some(P::Add | P::Subtract) => 11,
            Some(P::ShiftLeft) => 10,
            Some(P::Greater)
                if self.token()?.shift_half == 1 && self.cursor.peek(1)?.shift_half == 2 =>
            {
                op = Self::make_operator(P::ShiftRight)?;
                width = 2;
                10
            }
            Some(P::Less | P::Greater | P::LessEqual | P::GreaterEqual) => 9,
            Some(P::Equal | P::NotEqual) => 8,
            Some(P::BitAnd) => 7,
            Some(P::Xor) => 6,
            Some(P::BitOr) => 5,
            Some(P::LogicalAnd) => 4,
            Some(P::LogicalOr) => 3,
            _ => return Ok(None),
        };
        Ok(Some((op, precedence, width)))
    }
    fn binary(&mut self, minimum: u8) -> Result<ExprId, ParseError> {
        let left = self.unary()?;
        self.binary_rest(left, minimum)
    }
    pub(super) fn binary_rest(
        &mut self,
        mut left: ExprId,
        minimum: u8,
    ) -> Result<ExprId, ParseError> {
        while let Some((op, precedence, width)) = self.binary_operator()? {
            // An operator followed by the fold ellipsis ends this operand.
            if precedence < minimum || self.at_ahead(P::Ellipsis, width)? {
                break;
            }
            for _ in 0..width {
                self.cursor.advance()?;
            }
            let right = self.binary(precedence + 1)?;
            let location = self.tree.expressions.get(left).location;
            left = self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Binary {
                    op,
                    left,
                    right,
                    assignment: false,
                },
            })?;
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<ExprId, ParseError> {
        self.nested(Self::cast_operand)
    }
    fn cast_operand(&mut self) -> Result<ExprId, ParseError> {
        if self.at(P::LeftParen)? {
            self.parenthesized_or_cast()
        } else {
            self.unary_inner()
        }
    }
    fn parenthesized_or_cast(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        if self.at_ahead(P::Ellipsis, 1)? {
            let fold = self.without_angle(|parser| parser.left_fold(location))?;
            return self.postfix(fold);
        }
        if self.at_ahead(P::LeftBrace, 1)? {
            self.cursor.advance()?;
            let body = self.without_angle(Self::compound_statement)?;
            self.expect(P::RightParen)?;
            let expression = self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Statement(body),
            })?;
            return self.postfix(expression);
        }
        if self.name_starts_ahead(1)? {
            return self.parenthesized_from_name(location);
        }
        let cast = if self.type_specifier_start(1)? {
            self.attempt(|parser| {
                parser.cursor.advance()?;
                let ty = parser.without_angle(|parser| parser.type_id(false))?;
                if parser.at_compound_literal(ty)? {
                    return Ok(Operand::Compound(ty));
                }
                if !parser.accept(P::RightParen)? || !parser.operand_start()? {
                    return parser.decline_operand();
                }
                Ok(Operand::Type(ty))
            })?
        } else {
            None
        };
        if let Some(Operand::Compound(ty)) = cast {
            let literal = self.compound_literal(location, ty)?;
            return self.postfix(literal);
        }
        if let Some(Operand::Type(ty)) = cast {
            let operand = self.unary()?;
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Cast {
                    keyword: None,
                    ty,
                    operand,
                },
            });
        }
        // The ordinary parenthesized reading owns only its location and
        // child ID across recursion, rather than a general primary payload.
        self.cursor.advance()?;
        let expression = self.without_angle(|parser| {
            let inner = parser.expression()?;
            parser.parenthesized_end(location, inner)
        })?;
        self.postfix(expression)
    }
    fn at_compound_literal(&mut self, ty: TypeId) -> Result<bool, ParseError> {
        if self.environment != super::parser::Environment::Hosted
            || !self.at(P::RightParen)?
            || !self.at_ahead(P::LeftBrace, 1)?
        {
            return Ok(false);
        }
        Ok(self.tree.types.get(ty).declarator.is_none_or(|id| {
            let declarator = self.tree.declarators.get(id);
            declarator.pointers.is_empty()
                && declarator.nested.is_none()
                && declarator.suffixes.is_empty()
        }))
    }
    fn compound_literal(
        &mut self,
        location: SourceOffset,
        ty: TypeId,
    ) -> Result<ExprId, ParseError> {
        self.expect(P::RightParen)?;
        self.functional_cast(location, self.tree.types.get(ty).specifiers)
    }
    fn parenthesized_end(
        &mut self,
        location: SourceOffset,
        inner: ExprId,
    ) -> Result<ExprId, ParseError> {
        if self.fold_operator()?.is_some() {
            return self.fold_rest(location, inner);
        }
        self.expect(P::RightParen)?;
        let paren = self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Paren(inner),
        })?;
        Ok(paren)
    }
    fn fold_operator(&mut self) -> Result<Option<(Operator, usize)>, ParseError> {
        let operator = if let Some((op, _, width)) = self.binary_operator()? {
            Some((op, width))
        } else if let TokenKind::Simple(op) = self.token()?.kind {
            (op.token.as_punctuator() == Some(P::Comma) || assignment_operator(op))
                .then_some((op, 1))
        } else {
            None
        };
        Ok(match operator {
            Some((op, width)) if self.at_ahead(P::Ellipsis, width)? => Some((op, width)),
            _ => None,
        })
    }
    fn fold_rest(&mut self, location: SourceOffset, pattern: ExprId) -> Result<ExprId, ParseError> {
        if matches!(
            self.tree.expressions.get(pattern).kind,
            ExprKind::Binary { .. } | ExprKind::Conditional { .. } | ExprKind::Throw(_)
        ) {
            return self.fail();
        }
        let (mut op, width) = self
            .fold_operator()?
            .ok_or(ParseError::Invariant("fold operator"))?;
        op.alternative = false;
        for _ in 0..=width {
            self.cursor.advance()?;
        }
        let init = if self.at(P::RightParen)? {
            None
        } else {
            let (second, second_width) = self.fold_token()?;
            if second.token != op.token {
                return self.fail();
            }
            for _ in 0..second_width {
                self.cursor.advance()?;
            }
            Some(self.unary()?)
        };
        self.expect(P::RightParen)?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Fold {
                op,
                pattern,
                init,
                right: true,
            },
        })
    }
    fn fold_token(&mut self) -> Result<(Operator, usize), ParseError> {
        if let Some((op, _, width)) = self.binary_operator()? {
            Ok((op, width))
        } else if let TokenKind::Simple(op) = self.token()?.kind {
            Ok((op, 1))
        } else {
            self.fail()
        }
    }
    fn left_fold(&mut self, location: SourceOffset) -> Result<ExprId, ParseError> {
        self.cursor.advance()?;
        self.cursor.advance()?;
        let (mut op, width) = self.fold_token()?;
        op.alternative = false;
        for _ in 0..width {
            self.cursor.advance()?;
        }
        let pattern = self.unary()?;
        self.expect(P::RightParen)?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Fold {
                op,
                pattern,
                init: None,
                right: false,
            },
        })
    }
    pub(super) fn complex_component_keyword(&mut self) -> Result<Option<bool>, ParseError> {
        self.complex_component_ahead(0)
    }
    pub(super) fn complex_component_ahead(
        &mut self,
        ahead: usize,
    ) -> Result<Option<bool>, ParseError> {
        Ok(
            if let TokenKind::Identifier(name) = self.cursor.peek(ahead)?.kind {
                match self.cursor.name(name) {
                    b"__real__" | b"__real" => Some(false),
                    b"__imag__" | b"__imag" => Some(true),
                    _ => None,
                }
            } else {
                None
            },
        )
    }
    fn unary_inner(&mut self) -> Result<ExprId, ParseError> {
        self.stack_check()?;
        let location = self.location()?;
        if self.coroutine_keyword()? == Some(Coroutine::Await) {
            self.cursor.advance()?;
            let operand = self.unary()?;
            return self.coroutine_operation(Coroutine::Await, Some(operand), location);
        }
        if self.at(P::Scope)?
            && matches!(self.cursor.peek(1)?.kind, TokenKind::Simple(op) if matches!(op.token.as_keyword(), Some(K::New | K::Delete)))
        {
            return if matches!(self.cursor.peek(1)?.kind, TokenKind::Simple(op) if op.token.as_keyword() == Some(K::New))
            {
                self.new_expression()
            } else {
                self.delete_expression()
            };
        }
        if matches!(
            self.punctuator()?,
            Some(
                P::Increment
                    | P::Decrement
                    | P::Multiply
                    | P::BitAnd
                    | P::Add
                    | P::Subtract
                    | P::LogicalNot
                    | P::Complement
            )
        ) {
            let op = self.operator()?;
            let operand = self.unary()?;
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Unary { op, operand },
            });
        }
        if let Some(imaginary) = self.complex_component_keyword()? {
            self.cursor.advance()?;
            let operand = self.unary()?;
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::ComplexPart { imaginary, operand },
            });
        }
        let va_arg = if let Some(name) = self.va_arg {
            name
        } else {
            let name = self.cursor.intern_name(b"__builtin_va_arg")?;
            self.va_arg = Some(name);
            name
        };
        if matches!(self.token()?.kind, TokenKind::Identifier(name) if name == va_arg) {
            self.cursor.advance()?;
            self.expect(P::LeftParen)?;
            let (operand, ty) = self.without_angle(|parser| {
                let operand = parser.assignment_expression()?;
                parser.expect(P::Comma)?;
                let ty = parser.type_id(false)?;
                parser.expect(P::RightParen)?;
                Ok((operand, ty))
            })?;
            let expression = self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::VaArg { operand, ty },
            })?;
            return self.postfix(expression);
        }
        match self.keyword()? {
            Some(K::Sizeof) => self.sizeof_expression(location),
            Some(keyword @ (K::Alignof | K::Typeid | K::Noexcept)) => {
                self.type_trait(location, keyword)
            }
            Some(
                keyword @ (K::StaticCast | K::DynamicCast | K::ReinterpretCast | K::ConstCast),
            ) => {
                self.cursor.advance()?;
                self.expect(P::Less)?;
                let ty = self.with_angle(true, |parser| parser.type_id(true))?;
                self.expect(P::Greater)?;
                self.expect(P::LeftParen)?;
                let operand = self.without_angle(Self::expression)?;
                self.expect(P::RightParen)?;
                let expr = self.tree.expressions.add(Expression {
                    location,
                    kind: ExprKind::Cast {
                        keyword: Some(keyword),
                        ty,
                        operand,
                    },
                })?;
                self.postfix(expr)
            }
            Some(K::New) => self.new_expression(),
            Some(K::Delete) => self.delete_expression(),
            _ => {
                let primary = self.primary()?;
                self.postfix(primary)
            }
        }
    }
    fn operand_start(&mut self) -> Result<bool, ParseError> {
        Ok(matches!(
            self.token()?.kind,
            TokenKind::Identifier(_) | TokenKind::Literal(_)
        ) || matches!(
            self.punctuator()?,
            Some(
                P::LeftParen
                    | P::Scope
                    | P::LeftBracket
                    | P::Increment
                    | P::Decrement
                    | P::Multiply
                    | P::BitAnd
                    | P::Add
                    | P::Subtract
                    | P::LogicalNot
                    | P::Complement
            )
        ) || matches!(
            self.keyword()?,
            Some(
                K::True
                    | K::False
                    | K::Nullptr
                    | K::This
                    | K::Sizeof
                    | K::New
                    | K::Delete
                    | K::Alignof
                    | K::Noexcept
                    | K::Typeid
                    | K::StaticCast
                    | K::DynamicCast
                    | K::ReinterpretCast
                    | K::ConstCast
                    | K::Typename
                    | K::Decltype
                    | K::Operator
            )
        ) || self.keyword()?.is_some_and(Self::has_type_keyword))
    }
    fn new_expression(&mut self) -> Result<ExprId, ParseError> {
        self.without_angle(Self::new_expression_inner)
    }
    fn new_expression_inner(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        let global = self.accept(P::Scope)?;
        if !self.accept_keyword(K::New)? {
            return self.fail();
        }
        let mut placement = None;
        let mut parenthesized_type = false;
        let mut ty = None;
        if self.at(P::LeftParen)? {
            ty = self.attempt(|parser| {
                parser.cursor.advance()?;
                let ty = parser.type_id(false)?;
                parser.expect(P::RightParen)?;
                if parser.type_specifier_start(0)? {
                    // The oracle leaves this alternative without calling fail().
                    return Err(ParseError::Syntax {
                        location: parser.location()?,
                        message: "syntax error",
                    });
                }
                Ok(ty)
            })?;
            parenthesized_type = ty.is_some();
            if ty.is_none() {
                self.expect(P::LeftParen)?;
                placement = Some(self.expression_list(P::RightParen)?);
                if self.accept(P::LeftParen)? {
                    ty = Some(self.type_id(true)?);
                    parenthesized_type = true;
                    self.expect(P::RightParen)?;
                }
            }
        }
        let ty = if let Some(ty) = ty {
            ty
        } else {
            self.type_id_in_mode(true, super::declarators::Mode::New)?
        };
        let initializer = if matches!(self.punctuator()?, Some(P::LeftParen | P::LeftBrace)) {
            Some(self.initializer()?)
        } else {
            None
        };
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::New {
                global,
                placement,
                parenthesized_type,
                ty,
                initializer,
            },
        })
    }
    fn delete_expression(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        let global = self.accept(P::Scope)?;
        if !self.accept_keyword(K::Delete)? {
            return self.fail();
        }
        let array = self.at(P::LeftBracket)? && self.at_ahead(P::RightBracket, 1)?;
        if array {
            self.cursor.advance()?;
            self.cursor.advance()?;
        }
        let operand = self.unary()?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Delete {
                global,
                array,
                operand,
            },
        })
    }
    fn named_function_type(&self, ty: super::ast::TypeId) -> bool {
        let ty = self.tree.types.get(ty);
        let Some(declarator) = ty.declarator else {
            return false;
        };
        let declarator = self.tree.declarators.get(declarator);
        declarator.pointers.is_empty()
            && declarator.nested.is_none()
            && matches!(
                self.tree.suffixes.get(declarator.suffixes).first(),
                Some(super::ast::Suffix::Function { .. })
            )
            && self
                .tree
                .specs
                .get(ty.specifiers)
                .iter()
                .any(|specifier| matches!(specifier.kind, SpecifierKind::Named(_)))
    }
    fn named_operand(&mut self, function_types: bool) -> Result<Operand, ParseError> {
        let location = self.location()?;
        let (name, binding) = self.name()?;
        if self.is_type_name(name, binding, false, false)
            && let Some(ty) = self.attempt(|parser| {
                let ty = parser.type_from_name(name, location)?;
                parser.operand_type_end(ty, function_types)
            })?
        {
            return Ok(Operand::Type(ty));
        }
        let primary = self.primary_from_name(name, binding, location)?;
        Ok(Operand::Expression(self.continue_expression(primary)?))
    }
    fn operand_type_end(&mut self, ty: TypeId, function_types: bool) -> Result<TypeId, ParseError> {
        if !self.at(P::RightParen)? || (!function_types && self.named_function_type(ty)) {
            return self.decline_operand();
        }
        Ok(ty)
    }
    fn parenthesized_from_name(&mut self, location: SourceOffset) -> Result<ExprId, ParseError> {
        self.cursor.advance()?;
        let reading = self.without_angle(|parser| {
            let start = parser.location()?;
            let (name, binding) = parser.name()?;
            if parser.is_type_name(name, binding, false, false)
                && let Some(ty) = parser.attempt(|parser| {
                    let ty = parser.type_from_name(name, start)?;
                    if parser.at_compound_literal(ty)? {
                        return Ok(Operand::Compound(ty));
                    }
                    if !parser.accept(P::RightParen)? || !parser.operand_start()? {
                        return parser.decline_operand();
                    }
                    Ok(Operand::Type(ty))
                })?
            {
                return Ok(ty);
            }
            let primary = parser.primary_from_name(name, binding, start)?;
            let inner = parser.continue_expression(primary)?;
            Ok(Operand::Expression(
                parser.parenthesized_end(location, inner)?,
            ))
        })?;
        match reading {
            Operand::Type(ty) => {
                let operand = self.unary()?;
                self.tree.expressions.add(Expression {
                    location,
                    kind: ExprKind::Cast {
                        keyword: None,
                        ty,
                        operand,
                    },
                })
            }
            Operand::Expression(inner) => self.postfix(inner),
            Operand::Compound(ty) => {
                let literal = self.compound_literal(location, ty)?;
                self.postfix(literal)
            }
        }
    }
    fn type_trait(&mut self, location: SourceOffset, keyword: K) -> Result<ExprId, ParseError> {
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let reading = self.without_angle(|parser| {
            let named = parser.name_starts_ahead(0)?;
            let reading = if keyword == K::Noexcept {
                Operand::Expression(parser.expression()?)
            } else if named {
                parser.named_operand(false)?
            } else {
                let ty = if keyword == K::Alignof || parser.type_specifier_start(0)? {
                    parser.attempt(|parser| {
                        let ty = parser.type_id(false)?;
                        parser.operand_type_end(ty, keyword == K::Alignof)
                    })?
                } else {
                    None
                };
                if let Some(ty) = ty {
                    Operand::Type(ty)
                } else {
                    Operand::Expression(parser.expression()?)
                }
            };
            parser.expect(P::RightParen)?;
            Ok(reading)
        })?;
        let (ty, operand) = match reading {
            Operand::Type(ty) => (Some(ty), None),
            Operand::Compound(_) => return Err(ParseError::Invariant("compound type operand")),
            Operand::Expression(expression) => (None, Some(expression)),
        };
        let expr = self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::TypeTrait {
                keyword,
                ty,
                operand,
            },
        })?;
        if keyword == K::Typeid {
            self.postfix(expr)
        } else {
            Ok(expr)
        }
    }
    fn sizeof_expression(&mut self, location: SourceOffset) -> Result<ExprId, ParseError> {
        self.cursor.advance()?;
        if self.accept(P::Ellipsis)? {
            self.expect(P::LeftParen)?;
            let name = self.identifier()?;
            self.expect(P::RightParen)?;
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::SizeofPack(name),
            });
        }
        let (ty, operand, parenthesized) = self.sizeof_operand(false)?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Sizeof {
                ty,
                operand,
                parenthesized,
            },
        })
    }
    pub(super) fn typeof_operand(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
        self.cursor.advance()?;
        let (ty, operand, _) = self.sizeof_operand(true)?;
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Typeof { ty, operand },
        })
    }
    fn sizeof_operand(
        &mut self,
        function_types: bool,
    ) -> Result<(Option<TypeId>, Option<ExprId>, bool), ParseError> {
        // typeof reads a named T() as a function type-id; sizeof reads the
        // functional cast. Both retain the same bounded name/operand reading.
        if self.at(P::LeftParen)? && self.name_starts_ahead(1)? {
            let start = self.location()?;
            self.cursor.advance()?;
            let reading = self.without_angle(|parser| {
                let reading = parser.named_operand(function_types)?;
                parser.expect(P::RightParen)?;
                Ok(reading)
            })?;
            let (ty, operand, parenthesized) = match reading {
                Operand::Type(ty) => (Some(ty), None, false),
                Operand::Compound(_) => {
                    return Err(ParseError::Invariant("compound sizeof operand"));
                }
                Operand::Expression(inner) => {
                    let paren = self.tree.expressions.add(Expression {
                        location: start,
                        kind: ExprKind::Paren(inner),
                    })?;
                    let operand = self.postfix(paren)?;
                    if let ExprKind::Paren(inner) = self.tree.expressions.get(operand).kind {
                        (None, Some(inner), true)
                    } else {
                        (None, Some(operand), false)
                    }
                }
            };
            return Ok((ty, operand, parenthesized));
        }
        let mut parenthesized = false;
        let ty = if self.at(P::LeftParen)? && self.type_specifier_start(1)? {
            self.attempt(|parser| {
                parser.cursor.advance()?;
                let ty = parser.without_angle(|parser| parser.type_id(false))?;
                if !function_types && parser.named_function_type(ty) {
                    return Err(ParseError::Syntax {
                        location: parser.location()?,
                        message: "syntax error",
                    });
                }
                if !parser.accept(P::RightParen)? {
                    return parser.decline_operand();
                }
                Ok(ty)
            })?
        } else {
            None
        };
        let operand = if ty.is_none() {
            let operand = self.unary_inner()?;
            if let ExprKind::Paren(inner) = self.tree.expressions.get(operand).kind {
                parenthesized = true;
                Some(inner)
            } else {
                Some(operand)
            }
        } else {
            None
        };
        Ok((ty, operand, parenthesized))
    }
    pub(super) fn primary(&mut self) -> Result<ExprId, ParseError> {
        let token = self.token()?;
        let location = token.location;
        let kind = match token.kind {
            TokenKind::Literal(literal) => {
                self.cursor.advance()?;
                ExprKind::Literal(literal)
            }
            TokenKind::Identifier(_) => {
                if let Some(builtin) = self.at_value_builtin()? {
                    return self.value_builtin(builtin);
                }
                if self.at_integer_pack(0)? {
                    self.cursor.advance()?;
                    self.expect(P::LeftParen)?;
                    let operand = self.without_angle(Self::assignment_expression)?;
                    self.expect(P::RightParen)?;
                    return self.tree.expressions.add(Expression {
                        location,
                        kind: ExprKind::IntegerPack(operand),
                    });
                }
                if self.at_offsetof(0)? {
                    return self.offsetof_expression();
                }
                if let Some(predicate) = self.at_type_predicate(0)? {
                    return self.predicate_operand(predicate);
                }
                return self.primary_name(location);
            }
            TokenKind::Simple(op) => match op.token.as_keyword() {
                Some(keyword @ (K::True | K::False | K::Nullptr | K::This)) => {
                    self.cursor.advance()?;
                    ExprKind::Keyword(keyword)
                }
                Some(keyword) if Self::has_type_keyword(keyword) => {
                    let start = self.tree.specs.begin();
                    while let Some(keyword) =
                        self.keyword()?.filter(|&word| Self::has_type_keyword(word))
                    {
                        let location = self.location()?;
                        self.cursor.advance()?;
                        self.tree.specs.push(Specifier {
                            location,
                            kind: SpecifierKind::Keyword(keyword),
                        });
                    }
                    let specifiers = self.tree.specs.finish(start)?;
                    return self.functional_cast(location, specifiers);
                }
                Some(K::Decltype | K::Typename | K::Operator) => {
                    return self.primary_name(location);
                }
                _ => match op.token.as_punctuator() {
                    Some(P::LeftParen) => {
                        return self.without_angle(|parser| {
                            if parser.at_ahead(P::Ellipsis, 1)? {
                                parser.left_fold(location)
                            } else {
                                parser.cursor.advance()?;
                                let inner = parser.expression()?;
                                parser.parenthesized_end(location, inner)
                            }
                        });
                    }
                    Some(P::Scope) => return self.primary_name(location),
                    Some(P::LeftBrace) => return self.braced_list(),
                    Some(P::LeftBracket) => return self.lambda(),
                    _ => return self.fail(),
                },
            },
            TokenKind::End => return self.fail(),
        };
        self.tree.expressions.add(Expression { location, kind })
    }
    fn primary_name(&mut self, location: SourceOffset) -> Result<ExprId, ParseError> {
        let (name, binding) = self.name()?;
        self.primary_from_name(name, binding, location)
    }
    pub(super) fn primary_from_name(
        &mut self,
        name: super::ast::Name,
        binding: Option<super::scopes::Binding>,
        location: SourceOffset,
    ) -> Result<ExprId, ParseError> {
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let decltype = if !node.global
            && components.len() == 1
            && let super::ast::NameKind::Decltype(operand) = components[0].kind
        {
            Some(operand)
        } else {
            None
        };
        let known_type = node.typename_keyword
            || decltype.is_some()
            || binding.is_some_and(|binding| {
                matches!(
                    binding.category,
                    super::scopes::Category::Type | super::scopes::Category::TypeTemplate
                )
            });
        if (self.at(P::LeftParen)? && known_type)
            || (self.at(P::LeftBrace)?
                && (known_type || self.is_type_name(name, binding, false, false)))
        {
            let start = self.tree.specs.begin();
            self.tree.specs.push(Specifier {
                location,
                kind: decltype.map_or(SpecifierKind::Named(name), SpecifierKind::Decltype),
            });
            let specifiers = self.tree.specs.finish(start)?;
            return self.functional_cast(location, specifiers);
        }
        if decltype.is_some() || node.typename_keyword {
            return self.fail();
        }
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Id(name),
        })
    }
    fn functional_cast(
        &mut self,
        location: SourceOffset,
        specifiers: List<Specifier>,
    ) -> Result<ExprId, ParseError> {
        let (arguments, braced) = if self.at(P::LeftBrace)? {
            (List::default(), Some(self.braced_list()?))
        } else {
            self.expect(P::LeftParen)?;
            (self.expression_list(P::RightParen)?, None)
        };
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            },
        })
    }
    pub(super) fn postfix(&mut self, mut expr: ExprId) -> Result<ExprId, ParseError> {
        loop {
            let location = self.tree.expressions.get(expr).location;
            let kind = if self.accept(P::LeftParen)? {
                if self.reserved_call.is_none()
                    && let ExprKind::Id(name) = self.tree.expressions.get(expr).kind
                    && let [part] = self
                        .tree
                        .name_lists
                        .get(self.tree.names.get(name).components)
                    && let super::ast::NameKind::Identifier(identifier) = part.kind
                {
                    let spelling = self.cursor.name(identifier);
                    if spelling.starts_with(b"__builtin_")
                        && !crate::lex::builtins::is_provided(spelling)
                    {
                        self.reserved_call = Some(identifier);
                    }
                }
                ExprKind::Call {
                    callee: expr,
                    arguments: self.expression_list(P::RightParen)?,
                }
            } else if self.accept(P::LeftBracket)? {
                let index = self.without_angle(|parser| {
                    if parser.at(P::LeftBrace)? {
                        parser.braced_list()
                    } else {
                        parser.expression()
                    }
                })?;
                self.expect(P::RightBracket)?;
                ExprKind::Subscript { base: expr, index }
            } else if matches!(self.punctuator()?, Some(P::Dot | P::Arrow)) {
                let op = self.operator()?;
                ExprKind::Member {
                    base: expr,
                    op,
                    member: self.member_name()?.0,
                }
            } else if matches!(self.punctuator()?, Some(P::Increment | P::Decrement)) {
                ExprKind::Postfix {
                    op: self.operator()?,
                    operand: expr,
                }
            } else {
                return Ok(expr);
            };
            expr = self.tree.expressions.add(Expression { location, kind })?;
        }
    }
    pub(super) fn braced_list(&mut self) -> Result<ExprId, ParseError> {
        self.nested(|parser| {
            let location = parser.location()?;
            parser.expect(P::LeftBrace)?;
            let kind = parser.without_angle(Self::braced_elements)?;
            parser.tree.expressions.add(Expression { location, kind })
        })
    }
    fn braced_elements(&mut self) -> Result<ExprKind, ParseError> {
        let start = self.tree.expr_lists.begin();
        let mut trailing_comma = false;
        let designator_start = self.tree.designators.begin();
        let mut any_designator = false;
        let mut count = 0;
        while !self.at(P::RightBrace)? {
            let designator = self.initializer_designator()?;
            if designator.is_some() && !any_designator {
                for _ in 0..count {
                    self.tree.designators.push(None);
                }
                any_designator = true;
            }
            if any_designator {
                self.tree.designators.push(designator);
            }
            count += 1;
            let element = self.expanded_initializer()?;
            self.tree.expr_lists.push(element);
            if !self.accept(P::Comma)? {
                break;
            }
            trailing_comma = self.at(P::RightBrace)?;
        }
        self.expect(P::RightBrace)?;
        let designators = self.tree.designators.finish(designator_start)?;
        Ok(ExprKind::Braced {
            elements: self.tree.expr_lists.finish(start)?,
            designators: if any_designator {
                designators
            } else {
                List::default()
            },
            trailing_comma,
        })
    }

    fn initializer_designator(
        &mut self,
    ) -> Result<Option<crate::lex::identifiers::NameId>, ParseError> {
        if self.environment != super::parser::Environment::Hosted
            || !self.at(P::Dot)?
            || !matches!(self.cursor.peek(1)?.kind, TokenKind::Identifier(_))
            || !(self.at_ahead(P::Assign, 2)? || self.at_ahead(P::LeftBrace, 2)?)
        {
            return Ok(None);
        }
        self.cursor.advance()?;
        let name = self.identifier()?;
        self.accept(P::Assign)?;
        Ok(Some(name))
    }
    fn expanded_initializer(&mut self) -> Result<ExprId, ParseError> {
        let mentions = self.pack_mentions;
        let element = self.initializer_clause()?;
        if self.accept(P::Ellipsis)? {
            self.pack_mentions = mentions;
            let location = self.tree.expressions.get(element).location;
            self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::PackExpansion(element),
            })
        } else {
            Ok(element)
        }
    }
    pub(super) fn expression_list(&mut self, close: P) -> Result<List<ExprId>, ParseError> {
        self.without_angle(|parser| parser.expression_list_inner(close))
    }
    fn expression_list_inner(&mut self, close: P) -> Result<List<ExprId>, ParseError> {
        let start = self.tree.expr_lists.begin();
        while !self.at(close)? {
            let element = self.expanded_initializer()?;
            self.tree.expr_lists.push(element);
            if !self.accept(P::Comma)? {
                break;
            }
        }
        self.expect(close)?;
        self.tree.expr_lists.finish(start)
    }
}

pub(super) fn assignment_operator(op: Operator) -> bool {
    matches!(
        op.token.as_punctuator(),
        Some(
            P::Assign
                | P::AddAssign
                | P::SubtractAssign
                | P::MultiplyAssign
                | P::DivideAssign
                | P::RemainderAssign
                | P::ShiftLeftAssign
                | P::ShiftRightAssign
                | P::AndAssign
                | P::XorAssign
                | P::OrAssign
        )
    )
}
