// Translated from lower/placeholder_mangling.cpp::PlaceholderReader. See NOTICE.
//! ABI expressions consume the structural semantic key, without lookup or typing.
use super::{Error, Node, NodeId, Symbols};
use crate::{
    abi::vocabulary::Operation as O,
    lex::{identifiers::NameId, pp_lexer::Punctuator as P, vocabulary::Keyword},
    sema::lowering::{EntityId, KeyNode as N, KeyWord as W, TypeId, TypeView},
};

struct OperandReader<'a> {
    words: &'a [W],
    at: usize,
    object: Option<TypeId>,
}
enum OperandName {
    Type(TypeId),
    Entity(EntityId),
    Identifier(NameId),
}
struct OperandComponent {
    name: OperandName,
    arguments: Vec<TypeId>,
}
impl<'a> OperandReader<'a> {
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
    fn argument_count(&mut self) -> Result<usize, Error> {
        if matches!(self.words.get(self.at), Some(W::Node(N::Arguments))) {
            self.next()?;
            self.count()
        } else {
            Ok(0)
        }
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
    pub(super) fn unencoded_placeholder(&mut self, ty: TypeId) -> Result<NodeId, Error> {
        // Frozen abi_type_node retains an unmangled placeholder's zero-initialized
        // parameter position inside a larger type. Whole signature fallback is
        // separately owned by function_encoding.
        let value = self.graph.lower_value_argument(ty);
        let parameter = self.state.abi.intern(
            Node::TemplateParam {
                index: 0,
                level: if self.state.bound_pattern {
                    self.state.own_level
                } else {
                    0
                },
                substitutable: !value,
                kind: super::ParameterKind::Type,
            },
            &[],
        )?;
        if value {
            Ok(self
                .state
                .abi
                .intern(Node::ExpressionArgument(parameter), &[])?)
        } else {
            Ok(parameter)
        }
    }
    pub(super) fn declared_type_dependent(&self, ty: TypeId) -> Result<bool, Error> {
        let mut work = vec![ty];
        let mut visited = std::collections::HashSet::new();
        while let Some(ty) = work.pop() {
            if !visited.insert(ty) {
                continue;
            }
            if let Some(dependent) = self.graph.lower_operand_key_dependent(ty) {
                if dependent
                    || self
                        .graph
                        .lower_template_operand(ty)
                        .is_some_and(|(_, words)| words.is_empty())
                {
                    return Ok(true);
                }
                continue;
            }
            if let Some((owner, element)) = self.graph.lower_member_pointer_type(ty) {
                work.extend([owner, element]);
                continue;
            }
            match self.graph.lower_type(ty)? {
                TypeView::Pointer(element) | TypeView::Reference { element, .. } => {
                    work.push(element);
                }
                TypeView::Function {
                    result, parameters, ..
                } => {
                    work.push(result);
                    work.extend(parameters);
                }
                _ if self.graph.lower_dependent_type(ty) => return Ok(true),
                _ => (),
            }
        }
        Ok(false)
    }
    pub(super) fn placeholder_node(&mut self, ty: TypeId) -> Result<Option<NodeId>, Error> {
        let key = (ty, self.state.bound_pattern, self.state.own_level);
        if let Some(&node) = self.state.placeholders.get(&key) {
            return Ok(node);
        }
        if self.graph.lower_operand_key_dependent(ty) != Some(true) {
            self.state.placeholders.insert(key, None);
            return Ok(None);
        }
        self.state.placeholders.insert(key, None);
        let node = match self.placeholder_expression(ty) {
            Ok(node) => Some(node),
            // The frozen reader's unsupported production is ABI_NONE. Only
            // that result selects its concrete-specialization ABI fallback.
            Err(Error::Missing("dependent ABI expression form")) => None,
            Err(error) => {
                self.state.placeholders.remove(&key);
                return Err(error);
            }
        };
        self.state.placeholders.insert(key, node);
        Ok(node)
    }

    pub(super) fn unmangled_placeholder(&mut self, ty: TypeId) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&known) = self.state.unmangled.get(&ty) {
            return Ok(known);
        }
        let result = if self.graph.lower_operand_key_dependent(ty).is_some() {
            self.placeholder_node(ty)?.is_none()
        } else if let Some(arguments) = self.graph.lower_argument_pack(ty) {
            let mut found = false;
            for &argument in arguments {
                if self.unmangled_placeholder(argument)? {
                    found = true;
                    break;
                }
            }
            found
        } else if let Some(pattern) = self.graph.lower_pack_expansion(ty) {
            self.unmangled_placeholder(pattern)?
        } else if let Some((owner, member)) = self.graph.lower_member_pointer_type(ty) {
            self.unmangled_placeholder(owner)? || self.unmangled_placeholder(member)?
        } else {
            match self.graph.lower_type(ty)? {
                TypeView::Array { element, .. } => {
                    let bound = if let Some(operand) = self.graph.lower_array_operand(ty) {
                        self.placeholder_node(operand)?.is_none()
                    } else {
                        false
                    };
                    bound || self.unmangled_placeholder(element)?
                }
                TypeView::Pointer(element) | TypeView::Reference { element, .. } => {
                    self.unmangled_placeholder(element)?
                }
                TypeView::Class(class) if self.graph.lower_dependent_type(ty) => {
                    let mut found = false;
                    for argument in self.graph.lower_class_arguments(class).unwrap_or_default() {
                        if self.unmangled_placeholder(argument)? {
                            found = true;
                            break;
                        }
                    }
                    found
                }
                TypeView::Function {
                    result, parameters, ..
                } => {
                    let mut found = false;
                    for &parameter in parameters {
                        if self.unmangled_placeholder(parameter)? {
                            found = true;
                            break;
                        }
                    }
                    found || self.unmangled_placeholder(result)?
                }
                _ => false,
            }
        };
        self.state.unmangled.insert(ty, result);
        Ok(result)
    }
    pub(super) fn placeholder_expression(&mut self, ty: TypeId) -> Result<NodeId, Error> {
        let (_, words) = self.graph.lower_template_operand(ty).ok_or_else(missing)?;
        let mut reader = OperandReader {
            words,
            at: 0,
            object: self.graph.lower_operand_object(ty),
        };
        let result = self.operand_expression(&mut reader, 0)?;
        if reader.at != words.len() {
            return Err(missing());
        }
        Ok(result)
    }
    fn operand_keyword(&mut self, words: &mut OperandReader<'_>) -> Result<Node, Error> {
        let W::Keyword(Some(keyword)) = words.next()? else {
            return Err(missing());
        };
        Ok(match keyword {
            Keyword::This => {
                if !matches!(words.next()?, W::Node(N::This)) {
                    return Err(missing());
                }
                words.next()?;
                Node::ThisParam
            }
            Keyword::Nullptr => {
                let ty = self
                    .state
                    .abi
                    .intern(Node::Builtin(crate::abi::vocabulary::Builtin::Nullptr), &[])?;
                Node::NullptrLiteral(ty)
            }
            _ => return Err(missing()),
        })
    }
    fn operand_expression(
        &mut self,
        words: &mut OperandReader<'_>,
        depth: u8,
    ) -> Result<NodeId, Error> {
        self.check_stack()?;
        if depth > 64 {
            return Err(missing());
        }
        let node = match words.next()? {
            W::Transform(transform) if depth == 0 => {
                if words.count()? != 1 {
                    return Err(missing());
                }
                let argument = self.operand_type(words.ty()?)?;
                let name = self.state.abi.string(transform.spelling().as_bytes())?;
                return self
                    .state
                    .abi
                    .intern(Node::VendorType(name), &[argument])
                    .map_err(Error::from);
            }
            W::FloatingLiteral { ty, bits, .. } => Node::FloatingLiteral {
                ty: self.abi_type(*ty)?,
                bits: *bits,
            },
            W::Constant(ty, bits) => self.abi_literal(*ty, *bits)?,
            W::Node(N::Id) => return self.operand_name(words, None).map(|(node, _)| node),
            W::Node(N::Keyword) => self.operand_keyword(words)?,
            W::Node(N::Paren) => return self.operand_expression(words, depth + 1),
            W::Node(N::PackExpansion) => {
                return self.operand_operation(words, O::PackExpansion, 1, depth);
            }
            W::Node(N::Braced) => {
                let count = words.count()?;
                return self.operand_list(words, Node::BracedList(None), count, depth);
            }
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
                    return self.state.abi.intern(node, &[operand]).map_err(Error::from);
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
                    self.operand_name(words, Some(count))?
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
                let node = if parenthesized
                    && matches!(self.state.abi.get(callee), Node::UnresolvedName(_))
                {
                    Node::ParenthesizedCall
                } else {
                    Node::Call
                };
                return self.state.abi.intern(node, &arguments).map_err(Error::from);
            }
            W::Node(N::FunctionalCast) => {
                let count = words.count()?;
                let braced = words.flag()?;
                let ty = self.operand_type(words.ty()?)?;
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
                let member = self.state.abi.string(self.cursor.name(*name))?;
                let arguments = self.operand_arguments(words)?;
                return self
                    .state
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
            W::Node(N::SizeofPack) => return self.operand_pack_size(words),
            W::Node(N::New) => return self.operand_allocation(words, depth),
            W::Node(N::Delete) => {
                let global = words.flag()?;
                let array = words.flag()?;
                let operand = self.operand_expression(words, depth + 1)?;
                return self
                    .state
                    .abi
                    .intern(Node::Deallocation { array, global }, &[operand])
                    .map_err(Error::from);
            }
            W::Node(N::Throw) => {
                if words.flag()? {
                    Node::Rethrow
                } else {
                    return self.operand_operation(words, O::Throw, 1, depth);
                }
            }
            W::Node(N::Trait) => {
                let W::Keyword(Some(keyword)) = words.next()? else {
                    return Err(missing());
                };
                let of_type = words.flag()?;
                if of_type {
                    return match keyword {
                        Keyword::Alignof => self.operand_type_query(words.ty()?, true),
                        Keyword::Typeid => {
                            let ty = self.operand_type(words.ty()?)?;
                            self.state
                                .abi
                                .intern(Node::TypeidType(ty), &[])
                                .map_err(Error::from)
                        }
                        _ => Err(missing()),
                    };
                }
                let operation = match keyword {
                    Keyword::Alignof => O::AlignofExpression,
                    Keyword::Noexcept => O::Noexcept,
                    Keyword::Typeid => O::TypeidExpression,
                    _ => return Err(missing()),
                };
                return self.operand_operation(words, operation, 1, depth);
            }
            _ => return Err(missing()),
        };
        self.state.abi.intern(node, &[]).map_err(Error::from)
    }

    fn operand_allocation(
        &mut self,
        words: &mut OperandReader<'_>,
        depth: u8,
    ) -> Result<NodeId, Error> {
        let placed = words.count()?;
        let global = words.flag()?;
        let array = words.flag()?;
        let form = words.count()?;
        let count = words.count()?;
        let ty = self.operand_type(words.ty()?)?;
        // The array bound is consumed in source order but not encoded.
        if array {
            self.operand_expression(words, depth + 1)?;
        }
        let mut placement = Vec::new();
        for _ in 0..placed {
            placement.push(self.operand_expression(words, depth + 1)?);
        }
        let initializer = match form {
            0 => None,
            1 | 2 => Some(self.operand_list(
                words,
                if form == 1 {
                    Node::ParenthesizedList
                } else {
                    Node::BracedList(None)
                },
                count,
                depth,
            )?),
            _ => return Err(missing()),
        };
        self.state
            .abi
            .intern(
                Node::Allocation {
                    array,
                    global,
                    ty,
                    initializer,
                },
                &placement,
            )
            .map_err(Error::from)
    }

    fn operand_pack_size(&mut self, words: &mut OperandReader<'_>) -> Result<NodeId, Error> {
        let node = if matches!(words.words.get(words.at), Some(W::Flag(true))) {
            words.flag()?;
            let W::Parameter { index, level } = words.next()? else {
                return Err(missing());
            };
            words.ty()?;
            Node::FunctionParam {
                index: u32::try_from(*index).map_err(|_| missing())?,
                level: *level,
            }
        } else {
            let parameter = self.abi_parameter_level(words.ty()?)?;
            Node::TemplateParam {
                index: parameter.index,
                level: parameter.level,
                substitutable: false,
                kind: super::ParameterKind::Type,
            }
        };
        let pack = self.state.abi.intern(node, &[])?;
        self.state
            .abi
            .intern(Node::Operation(O::SizeofPack), &[pack])
            .map_err(Error::from)
    }

    fn operand_type(&mut self, ty: TypeId) -> Result<NodeId, Error> {
        if self.graph.lower_operand_key_dependent(ty) == Some(false)
            && self
                .graph
                .lower_template_operand(ty)
                .is_some_and(|(_, words)| !words.is_empty())
        {
            return Err(missing());
        }
        self.abi_type(ty)
    }

    fn operand_type_query(&mut self, mut ty: TypeId, alignment: bool) -> Result<NodeId, Error> {
        if !self.graph.lower_dependent_type(ty) && self.graph.lower_complete_object(ty)? {
            if let TypeView::Reference { element, .. } = self.graph.lower_type(ty)? {
                ty = element;
            }
            let (size, align) = super::super::types::layout(self.graph, ty)?;
            let target = self
                .state
                .abi
                .intern(Node::Builtin(super::B::UnsignedLong), &[])?;
            return self
                .state
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
        let ty = self.operand_type(ty)?;
        self.state
            .abi
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
            return self.operand_type(element);
        }
        let node = self.operand_type(written)?;
        let mut ty = written;
        let pointer = matches!(self.graph.lower_type(ty)?, TypeView::Pointer(_));
        while let TypeView::Pointer(element) = self.graph.lower_type(ty)? {
            ty = element;
        }
        let fixed = match self.graph.lower_type(ty)? {
            TypeView::Scalar(_) | TypeView::Enum(_) => true,
            TypeView::Class(class) => pointer && !self.graph.lower_templated_class(class),
            _ => false,
        };
        if fixed
            && let Node::Modified {
                modifier: super::Modifier::Qualified(_),
                operand,
            } = self.state.abi.get(node)
        {
            return Ok(operand);
        }
        Ok(node)
    }

    fn operand_braced(
        &mut self,
        words: &mut OperandReader<'_>,
        ty: Option<NodeId>,
        depth: u8,
    ) -> Result<NodeId, Error> {
        if !matches!(words.next()?, W::Node(N::Braced)) {
            return Err(missing());
        }
        let count = words.count()?;
        self.operand_list(words, Node::BracedList(ty), count, depth)
    }

    fn operand_list(
        &mut self,
        words: &mut OperandReader<'_>,
        node: Node,
        count: usize,
        depth: u8,
    ) -> Result<NodeId, Error> {
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            arguments.push(self.operand_expression(words, depth + 1)?);
        }
        self.state.abi.intern(node, &arguments).map_err(Error::from)
    }

    fn operand_operation(
        &mut self,
        words: &mut OperandReader<'_>,
        operation: O,
        count: usize,
        depth: u8,
    ) -> Result<NodeId, Error> {
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            arguments.push(self.operand_expression(words, depth + 1)?);
        }
        self.state
            .abi
            .intern(Node::Operation(operation), &arguments)
            .map_err(Error::from)
    }

    fn operand_cast_list(
        &mut self,
        words: &mut OperandReader<'_>,
        ty: NodeId,
        count: usize,
        depth: u8,
    ) -> Result<NodeId, Error> {
        if count == 1 {
            let operand = self.operand_expression(words, depth + 1)?;
            return self
                .state
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
        self.state
            .abi
            .intern(Node::ConversionList(ty), &arguments)
            .map_err(Error::from)
    }

    fn operand_name(
        &mut self,
        words: &mut OperandReader<'_>,
        call_arguments: Option<usize>,
    ) -> Result<(NodeId, Option<NodeId>), Error> {
        if !matches!(words.next()?, W::Node(N::Name)) {
            return Err(missing());
        }
        let count = words.count()?;
        let global = words.flag()?;
        let implicit_object = matches!(words.words.get(words.at), Some(W::Node(N::This)));
        if implicit_object {
            words.next()?;
            words.ty()?;
        }
        if count != 1 || global {
            return self.qualified_operand_name(
                words,
                count,
                global,
                implicit_object,
                call_arguments.is_some(),
            );
        }
        let (node, cast) = match words.next()? {
            W::Operator(operator) => {
                let array = words.flag()?;
                let terminal = self.abi_operator_terminal(
                    *operator,
                    array,
                    call_arguments.ok_or_else(missing)?,
                )?;
                (Node::UnresolvedName(terminal), None)
            }
            W::Parameter { index, level } => {
                words.ty()?;
                (
                    Node::FunctionParam {
                        index: u32::try_from(*index).map_err(|_| missing())?,
                        level: *level,
                    },
                    None,
                )
            }
            W::Type(ty)
                if self.graph.lower_value_argument(*ty)
                    && self.graph.lower_template_parameter(*ty).is_some() =>
            {
                let parameter = self.abi_parameter_level(*ty)?;
                return Ok((
                    self.state.abi.intern(
                        Node::TemplateParam {
                            index: parameter.index,
                            level: parameter.level,
                            substitutable: false,
                            kind: super::ParameterKind::Type,
                        },
                        &[],
                    )?,
                    None,
                ));
            }
            W::Type(ty) if call_arguments.is_some() => {
                let ty = self.operand_type(*ty)?;
                return Ok((ty, Some(ty)));
            }
            W::ResolvedFunction(entity) if !implicit_object => (
                Node::EntityReference(self.function_encoding(*entity)?),
                None,
            ),
            W::FreeName(name) | W::Identifier(name) | W::DependentCallee(_, name) => {
                (self.operand_function_name(*name, implicit_object)?, None)
            }
            W::Entity(entity) if self.graph.lower_operand_function(*entity) => {
                let name = self.graph.lower_local_name(*entity).ok_or_else(missing)?;
                (self.operand_function_name(name, implicit_object)?, None)
            }
            W::Entity(entity)
                if self.graph.lower_is_variable(*entity)
                    && self.dependent_operand_arguments(words)? =>
            {
                let name = self.graph.lower_local_name(*entity).ok_or_else(missing)?;
                (self.operand_function_name(name, implicit_object)?, None)
            }
            W::Entity(entity) => {
                return self.operand_entity(*entity, words.object, call_arguments.is_some());
            }
            _ => return Err(missing()),
        };
        let arguments = self.operand_arguments(words)?;
        Ok((self.state.abi.intern(node, &arguments)?, cast))
    }
    fn qualified_operand_name(
        &mut self,
        words: &mut OperandReader<'_>,
        count: usize,
        global: bool,
        implicit_object: bool,
        allow_type: bool,
    ) -> Result<(NodeId, Option<NodeId>), Error> {
        if count == 0 {
            return Err(missing());
        }
        // Read typed components once. Qualifier ABI nodes are demanded only by
        // a dependent resolution or overload set, after the terminal arguments.
        let mut components = Vec::new();
        for index in 0..count {
            let name = match words.next()? {
                W::Type(ty) if index == 0 && !global => OperandName::Type(*ty),
                W::Entity(entity) if index == 0 && !global => OperandName::Entity(*entity),
                W::Identifier(name) => OperandName::Identifier(*name),
                _ => return Err(missing()),
            };
            let arguments = (0..words.argument_count()?)
                .map(|_| words.ty())
                .collect::<Result<Vec<_>, _>>()?;
            components.push(OperandComponent { name, arguments });
        }
        let (last, qualifiers) = components.split_last().ok_or_else(missing)?;
        let OperandName::Identifier(terminal) = last.name else {
            return Err(missing());
        };
        let arguments = last
            .arguments
            .iter()
            .map(|&ty| self.operand_type(ty))
            .collect::<Result<Vec<_>, _>>()?;
        let parameter_root = matches!(
            components.first().map(|component| &component.name),
            Some(OperandName::Type(ty)) if self.graph.lower_template_parameter(*ty).is_some()
        );
        // The resolution distinguishes a dependent qualifier, a single
        // function, an overload set, and a named entity. Written levels alone
        // cannot decide which ABI production the frozen reader uses.
        let qualifier = match words.next()? {
            W::Type(ty) => Some(*ty),
            W::Node(N::Function) => match words.next()? {
                W::Entity(entity) if !implicit_object && !parameter_root => {
                    let encoding = self.function_encoding(*entity)?;
                    let node = self
                        .state
                        .abi
                        .intern(Node::EntityReference(encoding), &[])?;
                    return Ok((node, None));
                }
                W::Absent | W::Entity(_) => {
                    if implicit_object || parameter_root {
                        let member = self.state.abi.string(self.cursor.name(terminal))?;
                        let node = self
                            .state
                            .abi
                            .intern(Node::UnresolvedName(member), &arguments)?;
                        return Ok((node, None));
                    }
                    None
                }
                _ => return Err(missing()),
            },
            W::Entity(entity) => {
                if !arguments.is_empty() {
                    return Err(missing());
                }
                return self.operand_entity(*entity, words.object, allow_type);
            }
            _ => return Err(missing()),
        };
        let owner = self.operand_qualifier(qualifiers, global, implicit_object, qualifier)?;
        let member = self.state.abi.string(self.cursor.name(terminal))?;
        self.state
            .abi
            .intern(
                Node::ScopedMember {
                    owner,
                    member,
                    close_qualifier: false,
                },
                &arguments,
            )
            .map(|node| (node, None))
            .map_err(Error::from)
    }
    fn operand_qualifier(
        &mut self,
        components: &[OperandComponent],
        global: bool,
        implicit_object: bool,
        resolved: Option<TypeId>,
    ) -> Result<NodeId, Error> {
        let mut levels = Vec::new();
        let root = if implicit_object {
            self.qualifier_levels(resolved.ok_or_else(missing)?, &mut levels)?
        } else if let Some((first, rest)) = components.split_first() {
            let root = match first.name {
                OperandName::Type(ty) if self.graph.lower_template_parameter(ty).is_some() => {
                    Some(if rest.is_empty() {
                        self.operand_type(resolved.ok_or_else(missing)?)?
                    } else if first.arguments.is_empty() {
                        self.abi_type(ty)?
                    } else {
                        self.name_template_id(
                            self.graph.lower_named_template_id(ty, &first.arguments),
                        )?
                    })
                }
                OperandName::Type(ty) => self.qualifier_levels(ty, &mut levels)?,
                OperandName::Entity(entity)
                    if !self.graph.lower_operand_function(entity)
                        && !self.graph.lower_is_variable(entity) =>
                {
                    let name = self.graph.lower_local_name(entity).ok_or_else(missing)?;
                    levels.push(self.operand_qualifier_level(name, &first.arguments)?);
                    None
                }
                OperandName::Identifier(name) if global => {
                    levels.push(self.operand_qualifier_level(name, &first.arguments)?);
                    None
                }
                _ => return Err(missing()),
            };
            for component in rest {
                let OperandName::Identifier(name) = component.name else {
                    return Err(missing());
                };
                levels.push(self.operand_qualifier_level(name, &component.arguments)?);
            }
            root
        } else if global && resolved.is_none() {
            None
        } else {
            return Err(missing());
        };
        self.state
            .abi
            .intern(Node::UnresolvedPrefix { root, global }, &levels)
            .map_err(Error::from)
    }
    fn operand_qualifier_level(
        &mut self,
        name: NameId,
        arguments: &[TypeId],
    ) -> Result<NodeId, Error> {
        let arguments = arguments
            .iter()
            .map(|&ty| self.operand_type(ty))
            .collect::<Result<Vec<_>, _>>()?;
        let name = self.state.abi.string(self.cursor.name(name))?;
        self.state
            .abi
            .intern(Node::UnresolvedName(name), &arguments)
            .map_err(Error::from)
    }
    fn operand_function_name(
        &mut self,
        name: crate::lex::identifiers::NameId,
        implicit_object: bool,
    ) -> Result<Node, Error> {
        let member = self.state.abi.string(self.cursor.name(name))?;
        Ok(if implicit_object {
            let pointer = self.state.abi.intern(Node::ThisParam, &[])?;
            let object = self
                .state
                .abi
                .intern(Node::Operation(O::Dereference), &[pointer])?;
            Node::MemberAccess {
                operation: O::Dot,
                object,
                member,
            }
        } else {
            Node::UnresolvedName(member)
        })
    }
    fn operand_entity(
        &mut self,
        entity: EntityId,
        object: Option<TypeId>,
        allow_type: bool,
    ) -> Result<(NodeId, Option<NodeId>), Error> {
        if let Some(ty) = self.graph.lower_operand_expression_type(entity) {
            return self
                .placeholder_node(ty)?
                .map(|node| (node, None))
                .ok_or_else(missing);
        }
        if let Some(ty) = self.graph.lower_operand_type_name(entity) {
            if !allow_type {
                return Err(missing());
            }
            let ty = self.abi_type(ty)?;
            return Ok((ty, Some(ty)));
        }
        let node = if self.graph.lower_is_variable(entity) {
            if self.graph.lower_instance_member(entity) {
                if object.is_none() {
                    return Err(missing());
                }
                let name = self.graph.lower_local_name(entity).ok_or_else(missing)?;
                self.operand_function_name(name, true)?
            } else {
                Node::EntityReference(self.variable_abi_name(entity)?)
            }
        } else if let Some((ty, bits)) = self.graph.lower_operand_constant(entity) {
            self.abi_literal(ty, bits)?
        } else {
            return Err(missing());
        };
        self.state
            .abi
            .intern(node, &[])
            .map(|node| (node, None))
            .map_err(Error::from)
    }
    fn qualifier_levels(
        &mut self,
        mut ty: TypeId,
        levels: &mut Vec<NodeId>,
    ) -> Result<Option<NodeId>, Error> {
        let mut members = Vec::new();
        while let Some((qualifier, name)) = self.graph.lower_dependent_member(ty) {
            members.push(name);
            ty = qualifier;
        }
        if let Some(aliased) = self.graph.lower_aliased_type(ty) {
            ty = aliased;
        }
        let root = if let Some((name, arguments)) = self.graph.lower_qualifier_level(ty)? {
            let mut nodes = Vec::with_capacity(arguments.len());
            for &argument in arguments {
                nodes.push(self.abi_type(argument)?);
            }
            let name = self.state.abi.string(self.cursor.name(name))?;
            levels.push(self.state.abi.intern(Node::UnresolvedName(name), &nodes)?);
            None
        } else {
            Some(self.abi_type(ty)?)
        };
        for name in members.into_iter().rev() {
            let name = self.state.abi.string(self.cursor.name(name))?;
            levels.push(self.state.abi.intern(Node::UnresolvedName(name), &[])?);
        }
        Ok(root)
    }
    fn operand_arguments(&mut self, words: &mut OperandReader<'_>) -> Result<Vec<NodeId>, Error> {
        let count = words.argument_count()?;
        (0..count).map(|_| self.operand_type(words.ty()?)).collect()
    }
    fn dependent_operand_arguments(&self, words: &OperandReader<'_>) -> Result<bool, Error> {
        let Some([W::Node(N::Arguments), W::Count(count), tail @ ..]) = words.words.get(words.at..)
        else {
            return Ok(false);
        };
        let arguments = tail.get(..*count).ok_or_else(missing)?;
        for argument in arguments {
            let W::Type(ty) = argument else {
                return Err(missing());
            };
            if self.graph.lower_dependent_type(*ty) {
                return Ok(true);
            }
        }
        Ok(false)
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
