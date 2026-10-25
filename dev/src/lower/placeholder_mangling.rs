// Translated from lower/placeholder_mangling.cpp::PlaceholderReader. See NOTICE.
//! ABI expressions consume the structural semantic key, without lookup or typing.
use super::{Error, Node, NodeId, Symbols};
use crate::{
    abi::vocabulary::Operation as O,
    lex::{pp_lexer::Punctuator as P, vocabulary::Keyword},
    sema::lowering::{EntityId, KeyNode as N, KeyWord as W, TypeId, TypeView},
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
        let parameter = self.abi.intern(
            Node::TemplateParam {
                index: 0,
                level: if self.bound_pattern {
                    self.own_level
                } else {
                    0
                },
                substitutable: !value,
                kind: super::ParameterKind::Type,
            },
            &[],
        )?;
        if value {
            Ok(self.abi.intern(Node::ExpressionArgument(parameter), &[])?)
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
                if dependent {
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
        let key = (ty, self.bound_pattern, self.own_level);
        if let Some(&node) = self.placeholders.get(&key) {
            return Ok(node);
        }
        if self.graph.lower_operand_key_dependent(ty) != Some(true) {
            self.placeholders.insert(key, None);
            return Ok(None);
        }
        self.placeholders.insert(key, None);
        let (_, words) = self.graph.lower_template_operand(ty).ok_or_else(missing)?;
        let node = match self.placeholder_expression(words) {
            Ok(node) => Some(node),
            // The frozen reader's unsupported production is ABI_NONE. Only
            // that result selects its concrete-specialization ABI fallback.
            Err(Error::Missing("dependent ABI expression form")) => None,
            Err(error) => {
                self.placeholders.remove(&key);
                return Err(error);
            }
        };
        self.placeholders.insert(key, node);
        Ok(node)
    }

    pub(super) fn unmangled_placeholder(&mut self, ty: TypeId) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&known) = self.unmangled.get(&ty) {
            return Ok(known);
        }
        let result = if self.graph.lower_operand_key_dependent(ty).is_some() {
            self.placeholder_node(ty)?.is_none()
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
        self.unmangled.insert(ty, result);
        Ok(result)
    }
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
            W::Transform(transform) if depth == 0 => {
                if words.count()? != 1 {
                    return Err(missing());
                }
                let argument = self.abi_type(words.ty()?)?;
                let name = self.abi.string(transform.spelling().as_bytes())?;
                return self
                    .abi
                    .intern(Node::VendorType(name), &[argument])
                    .map_err(Error::from);
            }
            W::FloatingLiteral { ty, bits, .. } => Node::FloatingLiteral {
                ty: self.abi_type(*ty)?,
                bits: *bits,
            },
            W::Constant(ty, bits) => Node::Literal {
                ty: self.abi_type(*ty)?,
                bits: *bits,
            },
            W::Node(N::Id) => return self.operand_name(words).map(|(node, _)| node),
            W::Node(N::Keyword) => {
                let W::Keyword(Some(keyword)) = words.next()? else {
                    return Err(missing());
                };
                match keyword {
                    Keyword::This => {
                        if !matches!(words.next()?, W::Node(N::This)) {
                            return Err(missing());
                        }
                        words.next()?;
                        Node::ThisParam
                    }
                    _ => return Err(missing()),
                }
            }
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
            W::Node(N::SizeofPack) => return self.operand_pack_size(words),
            W::Node(N::New) => return self.operand_allocation(words, depth),
            W::Node(N::Delete) => {
                let global = words.flag()?;
                let array = words.flag()?;
                let operand = self.operand_expression(words, depth + 1)?;
                return self
                    .abi
                    .intern(Node::Deallocation { array, global }, &[operand])
                    .map_err(Error::from);
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

    fn operand_allocation(&mut self, words: &mut Words<'_>, depth: u8) -> Result<NodeId, Error> {
        let placed = words.count()?;
        let global = words.flag()?;
        let array = words.flag()?;
        let form = words.count()?;
        let count = words.count()?;
        let ty = self.abi_type(words.ty()?)?;
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
        self.abi
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

    fn operand_pack_size(&mut self, words: &mut Words<'_>) -> Result<NodeId, Error> {
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
        let pack = self.abi.intern(node, &[])?;
        self.abi
            .intern(Node::Operation(O::SizeofPack), &[pack])
            .map_err(Error::from)
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
        self.operand_list(words, Node::BracedList(ty), count, depth)
    }

    fn operand_list(
        &mut self,
        words: &mut Words<'_>,
        node: Node,
        count: usize,
        depth: u8,
    ) -> Result<NodeId, Error> {
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            arguments.push(self.operand_expression(words, depth + 1)?);
        }
        self.abi.intern(node, &arguments).map_err(Error::from)
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
        if !matches!(words.next()?, W::Node(N::Name)) {
            return Err(missing());
        }
        let count = words.count()?;
        let global = words.flag()?;
        if count != 1 || global {
            return self.qualified_operand_name(words, count, global);
        }
        let (node, cast) = match words.next()? {
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
                    self.abi.intern(
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
            W::Entity(entity)
                if self.graph.lower_is_variable(*entity)
                    && self.dependent_operand_arguments(words)? =>
            {
                let name = self.graph.lower_local_name(*entity).ok_or_else(missing)?;
                (
                    Node::UnresolvedName(self.abi.string(self.cursor.name(name))?),
                    None,
                )
            }
            W::Entity(entity) => return self.operand_entity(*entity),
            _ => return Err(missing()),
        };
        let arguments = self.operand_arguments(words)?;
        Ok((self.abi.intern(node, &arguments)?, cast))
    }
    fn qualified_operand_name(
        &mut self,
        words: &mut Words<'_>,
        count: usize,
        global: bool,
    ) -> Result<(NodeId, Option<NodeId>), Error> {
        if count == 0 {
            return Err(missing());
        }
        let mut root = None;
        let mut levels = Vec::new();
        let mut terminal = None;
        let mut arguments = Vec::new();
        let mut parameter_root = false;
        for index in 0..count {
            let word = words.next()?;
            let name = match word {
                W::Type(ty) if index == 0 && !global => {
                    parameter_root = self.graph.lower_template_parameter(*ty).is_some();
                    if parameter_root {
                        root = self.parameter_qualifier_root(*ty, words, count)?;
                    } else {
                        root = self.qualifier_levels(*ty, &mut levels)?;
                    }
                    None
                }
                W::Entity(entity) if index == 0 && !global => self.graph.lower_local_name(*entity),
                W::Identifier(name) => Some(*name),
                _ => return Err(missing()),
            };
            arguments = self.operand_arguments(words)?;
            if let Some(name) = name {
                let name = self.abi.string(self.cursor.name(name))?;
                if index + 1 == count {
                    terminal = Some(name);
                } else {
                    levels.push(self.abi.intern(Node::UnresolvedName(name), &arguments)?);
                }
            } else if !arguments.is_empty() {
                return Err(missing());
            }
        }
        // The resolution distinguishes a dependent qualifier, a single
        // function, an overload set, and a named entity. Written levels alone
        // cannot decide which ABI production the frozen reader uses.
        match words.next()? {
            // One parameter qualifier uses the resolved dependent qualifier
            // type, including its defaults. Longer qualifiers retain the
            // parameter template-id as written, followed by member levels.
            W::Type(ty) if parameter_root && count == 2 => {
                root = Some(self.abi_type(*ty)?);
            }
            W::Type(_) => (),
            W::Node(N::Function) => match words.next()? {
                W::Entity(entity) => {
                    let encoding = self.function_encoding(*entity)?;
                    let node = self.abi.intern(Node::EntityReference(encoding), &[])?;
                    return Ok((node, None));
                }
                W::Absent => (),
                _ => return Err(missing()),
            },
            W::Entity(entity) => {
                if !arguments.is_empty() {
                    return Err(missing());
                }
                return self.operand_entity(*entity);
            }
            _ => return Err(missing()),
        }
        let owner = self
            .abi
            .intern(Node::UnresolvedPrefix { root, global }, &levels)?;
        self.abi
            .intern(
                Node::ScopedMember {
                    owner,
                    member: terminal.ok_or_else(missing)?,
                    close_qualifier: false,
                },
                &arguments,
            )
            .map(|node| (node, None))
            .map_err(Error::from)
    }
    fn operand_entity(&mut self, entity: EntityId) -> Result<(NodeId, Option<NodeId>), Error> {
        if let Some(ty) = self.graph.lower_operand_expression_type(entity) {
            return self
                .placeholder_node(ty)?
                .map(|node| (node, None))
                .ok_or_else(missing);
        }
        if let Some(ty) = self.graph.lower_operand_type_name(entity) {
            let ty = self.abi_type(ty)?;
            return Ok((ty, Some(ty)));
        }
        let node = if self.graph.lower_is_variable(entity) {
            if self.graph.lower_instance_member(entity) {
                return Err(missing());
            }
            Node::EntityReference(self.variable_abi_name(entity)?)
        } else if let Some((ty, bits)) = self.graph.lower_operand_constant(entity) {
            Node::Literal {
                ty: self.abi_type(ty)?,
                bits,
            }
        } else {
            return Err(missing());
        };
        self.abi
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
            let name = self.abi.string(self.cursor.name(name))?;
            levels.push(self.abi.intern(Node::UnresolvedName(name), &nodes)?);
            None
        } else {
            Some(self.abi_type(ty)?)
        };
        for name in members.into_iter().rev() {
            let name = self.abi.string(self.cursor.name(name))?;
            levels.push(self.abi.intern(Node::UnresolvedName(name), &[])?);
        }
        Ok(root)
    }
    fn parameter_qualifier_root(
        &mut self,
        ty: TypeId,
        words: &mut Words<'_>,
        components: usize,
    ) -> Result<Option<NodeId>, Error> {
        let count = words.argument_count()?;
        // A single qualifier uses the recorded dependent type at resolution.
        // Consume and validate its written words without allocating ABI nodes.
        if components == 2 {
            for _ in 0..count {
                words.ty()?;
            }
            return Ok(None);
        }
        if count == 0 {
            return self.abi_type(ty).map(Some);
        }
        let arguments = (0..count)
            .map(|_| words.ty())
            .collect::<Result<Vec<_>, _>>()?;
        self.name_template_id(self.graph.lower_named_template_id(ty, &arguments))
            .map(Some)
    }
    fn operand_arguments(&mut self, words: &mut Words<'_>) -> Result<Vec<NodeId>, Error> {
        let count = words.argument_count()?;
        (0..count).map(|_| self.abi_type(words.ty()?)).collect()
    }
    fn dependent_operand_arguments(&self, words: &Words<'_>) -> Result<bool, Error> {
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
