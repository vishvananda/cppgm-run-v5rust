// Translated from lower/placeholder_mangling.cpp::PlaceholderReader. See NOTICE.
//! ABI expressions consume the structural semantic key, without lookup or typing.
use super::{Error, Node, NodeId, Symbols};
use crate::{
    abi::vocabulary::Operation as O,
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    sema::lowering::{KeyNode as N, KeyWord as W, TypeId, TypeView},
};

struct Words<'a> {
    words: &'a [W],
    at: usize,
}
impl<'a> Words<'a> {
    fn next(&mut self) -> Result<&'a W, Error> {
        let word = self.words.get(self.at).ok_or_else(missing)?;
        self.at += 1;
        Ok(word)
    }
    fn count(&mut self) -> Result<usize, Error> {
        let W::Count(count) = self.next()? else {
            return Err(missing());
        };
        Ok(*count)
    }
    fn flag(&mut self) -> Result<bool, Error> {
        let W::Flag(flag) = self.next()? else {
            return Err(missing());
        };
        Ok(*flag)
    }
    fn ty(&mut self) -> Result<TypeId, Error> {
        let W::Type(ty) = self.next()? else {
            return Err(missing());
        };
        Ok(*ty)
    }
}
fn missing() -> Error {
    Error::Missing("dependent ABI expression form")
}
impl Symbols<'_, '_> {
    pub(super) fn placeholder_expression(&mut self, words: &[W]) -> Result<NodeId, Error> {
        let mut reader = Words { words, at: 0 };
        let result = self.operand_expression(&mut reader, 0)?;
        if reader.at != words.len() {
            return Err(missing());
        }
        Ok(result)
    }
    fn operand_expression(&mut self, words: &mut Words<'_>, depth: u8) -> Result<NodeId, Error> {
        self.check_stack()?;
        if depth > 64 {
            return Err(missing());
        }
        let node = match words.next()? {
            W::Constant(ty, bits) => Node::Literal {
                ty: self.abi_type(*ty)?,
                bits: *bits,
            },
            W::Node(N::Id) => return self.operand_name(words).map(|(node, _)| node),
            W::Node(N::Paren) => return self.operand_expression(words, depth + 1),
            W::Node(N::Binary | N::Assign) => {
                let W::Operator(operator) = words.next()? else {
                    return Err(missing());
                };
                let operation =
                    binary(operator.as_punctuator().ok_or_else(missing)?).ok_or_else(missing)?;
                return self.operand_operation(words, operation, 2, depth);
            }
            W::Node(kind @ (N::Unary | N::Postfix)) => {
                let W::Operator(operator) = words.next()? else {
                    return Err(missing());
                };
                if let Some(op @ (P::Increment | P::Decrement)) = operator.as_punctuator() {
                    let operand = self.operand_expression(words, depth + 1)?;
                    let node = if *kind == N::Unary {
                        Node::PrefixStep {
                            increment: op == P::Increment,
                        }
                    } else {
                        Node::Operation(if op == P::Increment {
                            O::Increment
                        } else {
                            O::Decrement
                        })
                    };
                    return self.abi.intern(node, &[operand]).map_err(Error::from);
                }
                let operation = match operator.as_punctuator() {
                    Some(P::Add) => O::UnaryPlus,
                    Some(P::Subtract) => O::Negate,
                    Some(P::Multiply) => O::Dereference,
                    Some(P::BitAnd) => O::AddressOf,
                    Some(P::Complement) => O::Complement,
                    Some(P::LogicalNot) => O::LogicalNot,
                    _ => return Err(missing()),
                };
                return self.operand_operation(words, operation, 1, depth);
            }
            W::Node(N::Conditional) => {
                return self.operand_operation(words, O::Conditional, 3, depth);
            }
            W::Node(N::Subscript) => return self.operand_operation(words, O::Index, 2, depth),
            W::Node(N::Call) => {
                let count = words.count()?;
                let parenthesized = matches!(
                    words.words.get(words.at..),
                    Some([W::Node(N::Paren), W::Node(N::Id), ..])
                );
                if parenthesized {
                    words.next()?;
                }
                let (callee, cast) = if matches!(words.words.get(words.at), Some(W::Node(N::Id))) {
                    words.next()?;
                    self.operand_name(words)?
                } else {
                    (self.operand_expression(words, depth + 1)?, None)
                };
                if let Some(ty) = cast {
                    return self.operand_cast_list(words, ty, count, depth);
                }
                let mut arguments = vec![callee];
                for _ in 0..count {
                    arguments.push(self.operand_expression(words, depth + 1)?);
                }
                let node =
                    if parenthesized && matches!(self.abi.get(callee), Node::UnresolvedName(_)) {
                        Node::ParenthesizedCall
                    } else {
                        Node::Call
                    };
                return self.abi.intern(node, &arguments).map_err(Error::from);
            }
            W::Node(N::FunctionalCast) => {
                let count = words.count()?;
                let braced = words.flag()?;
                let ty = self.abi_type(words.ty()?)?;
                if braced {
                    return self.operand_braced(words, Some(ty), depth);
                }
                return self.operand_cast_list(words, ty, count, depth);
            }
            W::Node(N::Member) => {
                let W::Operator(operator) = words.next()? else {
                    return Err(missing());
                };
                let operation = if operator.as_punctuator() == Some(P::Arrow) {
                    O::Arrow
                } else {
                    O::Dot
                };
                let object = self.operand_expression(words, depth + 1)?;
                if !matches!(words.next()?, W::Node(N::Name))
                    || words.count()? != 1
                    || words.flag()?
                {
                    return Err(missing());
                }
                let W::Identifier(name) = words.next()? else {
                    return Err(missing());
                };
                let member = self.abi.string(self.cursor.name(*name))?;
                let arguments = self.operand_arguments(words)?;
                return self
                    .abi
                    .intern(
                        Node::MemberAccess {
                            operation,
                            object,
                            member,
                        },
                        &arguments,
                    )
                    .map_err(Error::from);
            }
            W::Node(N::Cast) => {
                let W::Keyword(keyword) = words.next()? else {
                    return Err(missing());
                };
                let operation = match keyword {
                    None => O::Conversion,
                    Some(Keyword::StaticCast) => O::StaticCast,
                    Some(Keyword::DynamicCast) => O::DynamicCast,
                    Some(Keyword::ConstCast) => O::ConstCast,
                    Some(Keyword::ReinterpretCast) => O::ReinterpretCast,
                    _ => return Err(missing()),
                };
                let ty = self.operand_cast_type(words.ty()?)?;
                let operand = self.operand_expression(words, depth + 1)?;
                Node::Cast {
                    operation,
                    ty,
                    operand,
                }
            }
            W::Node(N::Sizeof) => {
                words.next()?; // sizeof has no keyword.
                if words.flag()? {
                    return self.operand_type_query(words.ty()?, false);
                }
                return self.operand_operation(words, O::SizeofExpression, 1, depth);
            }
            W::Node(N::Trait) => {
                let W::Keyword(Some(keyword)) = words.next()? else {
                    return Err(missing());
                };
                let of_type = words.flag()?;
                if of_type {
                    if *keyword != Keyword::Alignof {
                        return Err(missing());
                    }
                    return self.operand_type_query(words.ty()?, true);
                }
                let operation = match keyword {
                    Keyword::Alignof => O::AlignofExpression,
                    Keyword::Noexcept => O::Noexcept,
                    _ => return Err(missing()),
                };
                return self.operand_operation(words, operation, 1, depth);
            }
            _ => return Err(missing()),
        };
        self.abi.intern(node, &[]).map_err(Error::from)
    }

    fn operand_type_query(&mut self, mut ty: TypeId, alignment: bool) -> Result<NodeId, Error> {
        if !self.graph.lower_dependent_type(ty) && self.graph.lower_complete_object(ty)? {
            if let TypeView::Reference { element, .. } = self.graph.lower_type(ty)? {
                ty = element;
            }
            let (size, align) = super::super::types::layout(self.graph, ty)?;
            let target = self
                .abi
                .intern(Node::Builtin(super::B::UnsignedLong), &[])?;
            return self
                .abi
                .intern(
                    Node::Literal {
                        ty: target,
                        bits: if alignment { 1u64 << align } else { size },
                    },
                    &[],
                )
                .map_err(Error::from);
        }
        let ty = self.abi_type(ty)?;
        self.abi
            .intern(
                if alignment {
                    Node::AlignofType(ty)
                } else {
                    Node::SizeofType(ty)
                },
                &[],
            )
            .map_err(Error::from)
    }

    fn operand_cast_type(&mut self, written: TypeId) -> Result<NodeId, Error> {
        if let TypeView::Reference { element, .. } = self.graph.lower_type(written)? {
            return self.abi_type(element);
        }
        let node = self.abi_type(written)?;
        let mut ty = written;
        while let TypeView::Pointer(element) = self.graph.lower_type(ty)? {
            ty = element;
        }
        let fixed = match self.graph.lower_type(ty)? {
            TypeView::Scalar(_) | TypeView::Enum(_) => true,
            TypeView::Class(class) => !self.graph.lower_templated_class(class),
            _ => false,
        };
        if fixed
            && let Node::Modified {
                modifier: super::Modifier::Qualified(_),
                operand,
            } = self.abi.get(node)
        {
            return Ok(operand);
        }
        Ok(node)
    }

    fn operand_braced(
        &mut self,
        words: &mut Words<'_>,
        ty: Option<NodeId>,
        depth: u8,
    ) -> Result<NodeId, Error> {
        if !matches!(words.next()?, W::Node(N::Braced)) {
            return Err(missing());
        }
        let count = words.count()?;
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            arguments.push(self.operand_expression(words, depth + 1)?);
        }
        self.abi
            .intern(Node::BracedList(ty), &arguments)
            .map_err(Error::from)
    }

    fn operand_operation(
        &mut self,
        words: &mut Words<'_>,
        operation: O,
        count: usize,
        depth: u8,
    ) -> Result<NodeId, Error> {
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            arguments.push(self.operand_expression(words, depth + 1)?);
        }
        self.abi
            .intern(Node::Operation(operation), &arguments)
            .map_err(Error::from)
    }

    fn operand_cast_list(
        &mut self,
        words: &mut Words<'_>,
        ty: NodeId,
        count: usize,
        depth: u8,
    ) -> Result<NodeId, Error> {
        if count == 1 {
            let operand = self.operand_expression(words, depth + 1)?;
            return self
                .abi
                .intern(
                    Node::Cast {
                        operation: O::Conversion,
                        ty,
                        operand,
                    },
                    &[],
                )
                .map_err(Error::from);
        }
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            arguments.push(self.operand_expression(words, depth + 1)?);
        }
        self.abi
            .intern(Node::ConversionList(ty), &arguments)
            .map_err(Error::from)
    }

    fn operand_name(&mut self, words: &mut Words<'_>) -> Result<(NodeId, Option<NodeId>), Error> {
        if !matches!(words.next()?, W::Node(N::Name)) || words.count()? != 1 || words.flag()? {
            return Err(missing());
        }
        let (node, cast) = match words.next()? {
            W::Parameter(index) => {
                words.ty()?;
                (
                    Node::FunctionParam(u32::try_from(*index).map_err(|_| missing())?),
                    None,
                )
            }
            W::Type(ty) => {
                let ty = self.abi_type(*ty)?;
                return Ok((ty, Some(ty)));
            }
            W::ResolvedFunction(entity) => (
                Node::EntityReference(self.function_encoding(*entity)?),
                None,
            ),
            W::FreeName(name) | W::Identifier(name) | W::DependentCallee(_, name) => (
                Node::UnresolvedName(self.abi.string(self.cursor.name(*name))?),
                None,
            ),
            W::Entity(entity) if self.graph.lower_operand_function(*entity) => {
                let name = self.graph.lower_local_name(*entity).ok_or_else(missing)?;
                (
                    Node::UnresolvedName(self.abi.string(self.cursor.name(name))?),
                    None,
                )
            }
            _ => return Err(missing()),
        };
        let arguments = self.operand_arguments(words)?;
        Ok((self.abi.intern(node, &arguments)?, cast))
    }
    fn operand_arguments(&mut self, words: &mut Words<'_>) -> Result<Vec<NodeId>, Error> {
        let mut arguments = Vec::new();
        if matches!(words.words.get(words.at), Some(W::Node(N::Arguments))) {
            words.next()?;
            let count = words.count()?;
            for _ in 0..count {
                arguments.push(self.abi_type(words.ty()?)?);
            }
        }
        Ok(arguments)
    }
}

fn binary(operator: P) -> Option<O> {
    Some(match operator {
        P::Add => O::Add,
        P::Subtract => O::Subtract,
        P::Multiply => O::Multiply,
        P::Divide => O::Divide,
        P::Remainder => O::Remainder,
        P::BitAnd => O::BitAnd,
        P::BitOr => O::BitOr,
        P::Xor => O::BitXor,
        P::Assign => O::Assign,
        P::AddAssign => O::AddAssign,
        P::SubtractAssign => O::SubtractAssign,
        P::MultiplyAssign => O::MultiplyAssign,
        P::DivideAssign => O::DivideAssign,
        P::RemainderAssign => O::RemainderAssign,
        P::AndAssign => O::AndAssign,
        P::OrAssign => O::OrAssign,
        P::XorAssign => O::XorAssign,
        P::ShiftLeft => O::ShiftLeft,
        P::ShiftRight => O::ShiftRight,
        P::ShiftLeftAssign => O::ShiftLeftAssign,
        P::ShiftRightAssign => O::ShiftRightAssign,
        P::Equal => O::Equal,
        P::NotEqual => O::NotEqual,
        P::Less => O::Less,
        P::Greater => O::Greater,
        P::LessEqual => O::LessEqual,
        P::GreaterEqual => O::GreaterEqual,
        P::LogicalAnd => O::LogicalAnd,
        P::LogicalOr => O::LogicalOr,
        P::Comma => O::Comma,
        P::DotStar => O::DotMember,
        P::ArrowStar => O::ArrowMember,
        _ => return None,
    })
}
