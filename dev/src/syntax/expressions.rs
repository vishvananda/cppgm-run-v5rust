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
enum Operand {
    Type(TypeId),
    Expression(ExprId),
}
impl Parser<'_> {
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
        while self.accept(P::Comma)? {
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
        self.nested(|parser| {
            if parser.keyword()? == Some(K::Throw) {
                let location = parser.location()?;
                parser.cursor.advance()?;
                let operand = if parser.operand_start()? {
                    Some(parser.assignment_expression()?)
                } else {
                    None
                };
                return parser.tree.expressions.add(Expression {
                    location,
                    kind: ExprKind::Throw(operand),
                });
            }
            let left = parser.binary(3)?;
            parser.assignment_rest(left)
        })
    }
    pub(super) fn assignment_rest(&mut self, left: ExprId) -> Result<ExprId, ParseError> {
        let location = self.tree.expressions.get(left).location;
        if self.accept(P::Question)? {
            let then_expr = self.without_angle(Self::expression)?;
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
        ) {
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
        self.nested(Self::unary_inner)
    }
    fn unary_inner(&mut self) -> Result<ExprId, ParseError> {
        let location = self.location()?;
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
        if self.at(P::LeftParen)? {
            if self.name_starts_ahead(1)? {
                return self.parenthesized_from_name(location);
            }
            let cast = self.attempt(|parser| {
                parser.cursor.advance()?;
                let ty = parser.without_angle(|parser| parser.type_id(false))?;
                if !parser.accept(P::RightParen)? || !parser.operand_start()? {
                    return parser.decline_operand();
                }
                Ok(ty)
            })?;
            if let Some(ty) = cast {
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
                let ty = self.type_id(true)?;
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
    fn type_specifier_start(&mut self) -> Result<bool, ParseError> {
        Ok(matches!(self.token()?.kind, TokenKind::Identifier(_))
            || self.at(P::Scope)?
            || matches!(
                self.keyword()?,
                Some(
                    K::Const
                        | K::Volatile
                        | K::Typename
                        | K::Decltype
                        | K::Class
                        | K::Struct
                        | K::Union
                        | K::Enum
                )
            )
            || self.keyword()?.is_some_and(Self::has_type_keyword))
    }
    fn new_expression(&mut self) -> Result<ExprId, ParseError> {
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
                if parser.type_specifier_start()? {
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
    fn name_starts_ahead(&mut self, ahead: usize) -> Result<bool, ParseError> {
        Ok(
            matches!(self.cursor.peek(ahead)?.kind, TokenKind::Identifier(_))
                || (self.at_ahead(P::Scope, ahead)?
                    && matches!(self.cursor.peek(ahead + 1)?.kind, TokenKind::Identifier(_))),
        )
    }
    fn decline_operand<T>(&mut self) -> Result<T, ParseError> {
        // A successfully parsed type followed by the wrong delimiter declines
        // the optional reading without recording a new furthest syntax error.
        Err(ParseError::Syntax {
            location: self.location()?,
            message: "syntax error",
        })
    }
    fn named_operand(&mut self) -> Result<Operand, ParseError> {
        let location = self.location()?;
        let (name, binding) = self.name()?;
        if self.is_type_name(name, binding, false, false)
            && let Some(ty) = self.attempt(|parser| {
                let ty = parser.type_from_name(name, location)?;
                parser.operand_type_end(ty)
            })?
        {
            return Ok(Operand::Type(ty));
        }
        let primary = self.primary_from_name(name, binding, location)?;
        Ok(Operand::Expression(self.continue_expression(primary)?))
    }
    fn operand_type(&mut self) -> Result<TypeId, ParseError> {
        let ty = self.type_id(false)?;
        self.operand_type_end(ty)
    }
    fn operand_type_end(&mut self, ty: TypeId) -> Result<TypeId, ParseError> {
        if !self.at(P::RightParen)? || self.named_function_type(ty) {
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
                    if !parser.accept(P::RightParen)? || !parser.operand_start()? {
                        return parser.decline_operand();
                    }
                    Ok(ty)
                })?
            {
                return Ok(Operand::Type(ty));
            }
            let primary = parser.primary_from_name(name, binding, start)?;
            let inner = parser.continue_expression(primary)?;
            parser.expect(P::RightParen)?;
            Ok(Operand::Expression(inner))
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
            Operand::Expression(inner) => {
                let paren = self.tree.expressions.add(Expression {
                    location,
                    kind: ExprKind::Paren(inner),
                })?;
                self.postfix(paren)
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
                parser.named_operand()?
            } else {
                let ty = parser.attempt(Self::operand_type)?;
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
        if self.at(P::LeftParen)? && self.name_starts_ahead(1)? {
            let start = self.location()?;
            self.cursor.advance()?;
            let reading = self.without_angle(|parser| {
                let reading = parser.named_operand()?;
                parser.expect(P::RightParen)?;
                Ok(reading)
            })?;
            let (ty, operand, parenthesized) = match reading {
                Operand::Type(ty) => (Some(ty), None, false),
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
            return self.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Sizeof {
                    ty,
                    operand,
                    parenthesized,
                },
            });
        }
        let mut parenthesized = false;
        let ty = if self.at(P::LeftParen)? {
            self.attempt(|parser| {
                parser.cursor.advance()?;
                let ty = parser.without_angle(|parser| parser.type_id(false))?;
                if parser.named_function_type(ty) {
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
            let operand = self.unary()?;
            if let ExprKind::Paren(inner) = self.tree.expressions.get(operand).kind {
                parenthesized = true;
                Some(inner)
            } else {
                Some(operand)
            }
        } else {
            None
        };
        self.tree.expressions.add(Expression {
            location,
            kind: ExprKind::Sizeof {
                ty,
                operand,
                parenthesized,
            },
        })
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
                return self.primary_name(location);
            }
            TokenKind::Simple(op) => match op.token.as_keyword() {
                Some(keyword @ (K::True | K::False | K::Nullptr | K::This)) => {
                    self.cursor.advance()?;
                    ExprKind::Keyword(keyword)
                }
                Some(keyword) if Self::has_type_keyword(keyword) => {
                    let specifiers = self.specifier_sequence(true, true)?;
                    return self.functional_cast(location, specifiers);
                }
                Some(K::Decltype | K::Typename | K::Operator) => {
                    return self.primary_name(location);
                }
                _ => match op.token.as_punctuator() {
                    Some(P::LeftParen) => {
                        self.cursor.advance()?;
                        let inner = self.without_angle(Self::expression)?;
                        self.expect(P::RightParen)?;
                        ExprKind::Paren(inner)
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
                ExprKind::Call {
                    callee: expr,
                    arguments: self.expression_list(P::RightParen)?,
                }
            } else if self.accept(P::LeftBracket)? {
                let index = self.without_angle(Self::expression)?;
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
            let saved_angle = parser.angle;
            parser.angle = false;
            let start = parser.tree.expr_lists.begin();
            let mut trailing_comma = false;
            while !parser.at(P::RightBrace)? {
                let element = parser.expanded_initializer()?;
                parser.tree.expr_lists.push(element);
                if !parser.accept(P::Comma)? {
                    break;
                }
                trailing_comma = parser.at(P::RightBrace)?;
            }
            parser.expect(P::RightBrace)?;
            let elements = parser.tree.expr_lists.finish(start)?;
            parser.angle = saved_angle;
            parser.tree.expressions.add(Expression {
                location,
                kind: ExprKind::Braced {
                    elements,
                    trailing_comma,
                },
            })
        })
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
