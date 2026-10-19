// Translated from sema/constant_construction.cpp and lower/constant_lowering.cpp.
// See NOTICE.
//! Evaluated value zeros differ from omitted aggregate members in static data spelling.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    constructor_initialization::ConstructorArguments,
    expression_facts::ClassInitialization,
    graph::{EntityId, ScopeId},
    static_arrays::{Item, append_zero},
    static_initialization::Value,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast, types::FundamentalType as F};
const VALUE_BUDGET: u64 = 1 << 22;

struct EvaluatedCall {
    constructor: ConstructorId,
    arguments: super::call_arguments::Arguments,
    zeroed: bool,
    primary: bool,
}
#[derive(Default)]
struct Placements {
    items: Vec<(u64, u64, Item)>,
    failure: Option<usize>,
}
impl Placements {
    fn push(&mut self, item: (u64, u64, Item)) {
        self.items.push(item);
    }
    fn unsupported(&mut self) {
        self.failure.get_or_insert(self.items.len());
    }
}
pub(super) struct EvaluatedImage {
    pub image: Option<Vec<Item>>,
    pub demands: Vec<Item>,
}

impl Analyzer<'_> {
    pub(super) fn evaluated_polymorphic_object(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
    ) -> Result<Option<EvaluatedImage>, Error> {
        let mut placements = Placements::default();
        let ty = self.graph.lower_entity_type(entity)?;
        if self.static_object_leaves(ty, 0, &mut std::collections::HashMap::new())? > VALUE_BUDGET {
            return Ok(None);
        }
        let selected = if let Some(constructor) = self.graph.lower_construction(entity) {
            Some(constructor)
        } else if let Some(expr) = initializer.and_then(|i| self.single_initializer(i)) {
            match self.graph.lower_class_copy(expr)? {
                Some(super::copy_initialization::CopyAction::Constructor {
                    constructor, ..
                }) => Some(constructor),
                _ => None,
            }
        } else {
            None
        };
        if !self.evaluated_object(
            ty,
            initializer,
            (selected, false),
            (scope, 0),
            &mut placements,
        )? {
            return Ok(None);
        }
        if let Some(first) = placements.failure {
            return Ok(Some(EvaluatedImage {
                image: None,
                demands: placements.items[..first]
                    .iter()
                    .filter_map(|&(_, _, item)| {
                        matches!(
                            item,
                            Item::Table(_)
                                | Item::Scalar(
                                    _,
                                    Value::Address(..) | Value::Literal(..) | Value::Temporary(..)
                                )
                        )
                        .then_some(item)
                    })
                    .collect(),
            }));
        }
        placements.items.sort_by_key(|&(at, _, _)| at);
        let size = self
            .type_size(self.graph.lower_entity_type(entity)?, SourceOffset(0))?
            .0;
        let mut cursor = 0;
        let mut image = Vec::with_capacity(placements.items.len());
        for (at, size, item) in placements.items {
            if at < cursor {
                if matches!(item, Item::BitUnit(..)) {
                    continue;
                }
                return Ok(None);
            }
            append_zero(&mut image, at - cursor)?;
            image.push(item);
            cursor = at
                .checked_add(size)
                .ok_or(Error::Limit("evaluated object image"))?;
        }
        let Some(tail) = size.checked_sub(cursor) else {
            return Ok(None);
        };
        append_zero(&mut image, tail)?;
        Ok(Some(EvaluatedImage {
            image: Some(image),
            demands: Vec::new(),
        }))
    }
    fn static_object_leaves(
        &self,
        mut ty: TypeId,
        depth: u32,
        memo: &mut std::collections::HashMap<(TypeId, u32), u64>,
    ) -> Result<u64, Error> {
        if let Some(&count) = memo.get(&(ty, depth)) {
            return Ok(count);
        }
        let key = (ty, depth);
        let mut count = 1_u64;
        while let TypeKind::Array { element, bound, .. } = self.graph.types.get(ty).kind {
            count = count.saturating_mul(bound.max(1));
            if count > VALUE_BUDGET {
                return Ok(VALUE_BUDGET + 1);
            }
            ty = element;
        }
        if let TypeKind::Class(class) = self.graph.types.get(ty).kind
            && depth <= 64
        {
            let mut each = 1_u64;
            for base in &self.graph.entity(class).bases {
                each = each.saturating_add(self.static_object_leaves(
                    self.graph.lower_entity_type(base.class)?,
                    depth + 1,
                    memo,
                )?);
                if each > VALUE_BUDGET {
                    break;
                }
            }
            for &member in &self.graph.entity(class).members {
                if each > VALUE_BUDGET {
                    break;
                }
                each = each.saturating_add(self.static_object_leaves(
                    self.graph.lower_entity_type(member)?,
                    depth + 1,
                    memo,
                )?);
            }
            count = count.saturating_mul(each).min(VALUE_BUDGET + 1);
        }
        memo.insert(key, count);
        Ok(count)
    }
    fn evaluated_object(
        &mut self,
        ty: TypeId,
        initializer: Option<ast::Initializer>,
        (selected, value_initialized): (Option<ConstructorId>, bool),
        (scope, at): (ScopeId, u64),
        out: &mut Placements,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if let TypeKind::Array { element, bound, .. } = self.graph.types.get(ty).kind {
            // An absent array initializer is not an evaluated value-initialization.
            let arguments = if initializer.is_none() && value_initialized {
                super::call_arguments::Arguments::Empty
            } else if let Some(arguments) =
                initializer.and_then(|i| self.evaluated_arguments(Some(i)))
            {
                arguments
            } else {
                return Ok(false);
            };
            let size = self.type_size(element, SourceOffset(0))?.0;
            for index in 0..bound {
                let expr = usize::try_from(index)
                    .ok()
                    .and_then(|i| arguments.as_slice().get(i))
                    .copied();
                let written = expr.map(|expr| ast::Initializer {
                    kind: ast::InitializerKind::Braced(expr),
                    location: self.tree.expressions.get(expr).location,
                });
                if !self.evaluated_object(
                    element,
                    written,
                    (selected, true),
                    (scope, at.wrapping_add(index.wrapping_mul(size))),
                    out,
                )? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(false);
        };
        let Some(constructor) = selected.or(self.graph.lower_ready_default_constructor(class))
        else {
            return Ok(false);
        };
        if self.graph.constructors.get(constructor).copy.is_some() {
            let Some(expr) = initializer.and_then(|i| self.single_initializer(i)) else {
                return Ok(false);
            };
            match self.constant_place(expr, scope) {
                Ok(_) => {}
                Err(Error::NonConstant { .. } | Error::Missing(_)) => return Ok(false),
                Err(error) => return Err(error),
            }
            return self.evaluated_empty_copy(constructor, at, false, out);
        }
        if let Some(expr) = initializer.and_then(|i| self.single_initializer(i))
            && let Some(call) = self.evaluated_prvalue(expr)?
        {
            return self.evaluated_constructor(&call, (scope, at), out, 0);
        }
        let Some(arguments) = self.evaluated_arguments(initializer) else {
            return Ok(false);
        };
        let call = EvaluatedCall {
            constructor,
            primary: false,
            zeroed: (initializer.is_some() || value_initialized)
                && arguments.as_slice().is_empty()
                && self.graph.lower_constructor(constructor).zero_before_value,
            arguments,
        };
        self.evaluated_constructor(&call, (scope, at), out, 0)
    }
    fn evaluated_prvalue(&self, mut expr: ast::ExprId) -> Result<Option<EvaluatedCall>, Error> {
        loop {
            let facts = self.graph.expression_facts.get(expr)?;
            match facts.class_initialization {
                Some(ClassInitialization::FromExpression(source)) => expr = source,
                Some(ClassInitialization::Constructed(arguments)) => {
                    let Some(constructor) = facts.constructor else {
                        return Ok(None);
                    };
                    let arguments = match arguments {
                        ConstructorArguments::Values(values) => self.tree.expr_lists.get(values),
                        ConstructorArguments::Value => &[],
                        ConstructorArguments::Single(_) => return Ok(None),
                    };
                    let arguments = super::call_arguments::Arguments::new(arguments);
                    return Ok(Some(EvaluatedCall {
                        constructor,
                        primary: false,
                        zeroed: arguments.as_slice().is_empty()
                            && self.graph.lower_constructor(constructor).zero_before_value,
                        arguments,
                    }));
                }
                _ => return Ok(None),
            }
        }
    }
    fn evaluated_empty_copy(
        &mut self,
        constructor: ConstructorId,
        at: u64,
        primary: bool,
        out: &mut Placements,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let plan = self.graph.constructors.get(constructor);
        let class = plan.class;
        let Some(actions) = &plan.copy else {
            return Ok(false);
        };
        if self.graph.lower_polymorphic(class) && !primary {
            out.push((at, 8, Item::Table(class)));
        }
        for index in 0..actions.len() {
            let action = self
                .graph
                .constructors
                .get(constructor)
                .copy
                .as_ref()
                .ok_or(Error::Invariant("copy image lost actions"))?[index];
            let Some(child) = action.constructor else {
                // A scalar read of an ordinary non-constant source is not a
                // core constant expression. Empty subobjects read no value.
                return Ok(false);
            };
            let (offset, primary) = match action.target {
                super::copy_construction::CopyTarget::Base(index) => {
                    let base = &self.graph.entity(class).bases[index as usize];
                    let offset = base
                        .offset
                        .ok_or(Error::Invariant("copy image base layout"))?;
                    (
                        offset,
                        self.graph.lower_polymorphic(base.class) && offset == 0,
                    )
                }
                super::copy_construction::CopyTarget::Member(member) => (
                    self.graph
                        .entity(member)
                        .member_offset
                        .ok_or(Error::Invariant("copy image member layout"))?,
                    false,
                ),
            };
            if !self.evaluated_empty_copy(child, at.wrapping_add(offset), primary, out)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn evaluated_arguments(
        &self,
        initializer: Option<ast::Initializer>,
    ) -> Option<super::call_arguments::Arguments> {
        let values = match initializer.map(|i| i.kind) {
            None => Some(&[][..]),
            Some(ast::InitializerKind::Parenthesized(values)) => {
                Some(self.tree.expr_lists.get(values))
            }
            Some(ast::InitializerKind::Braced(id) | ast::InitializerKind::Assign(id)) => {
                if let ast::ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
                    Some(self.tree.expr_lists.get(elements))
                } else {
                    None
                }
            }
            _ => None,
        };
        values.map(super::call_arguments::Arguments::new)
    }
    fn evaluated_constructor(
        &mut self,
        call: &EvaluatedCall,
        (scope, at): (ScopeId, u64),
        out: &mut Placements,
        depth: u32,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let plan = self.graph.constructors.get(call.constructor);
        if depth >= 512
            || plan.delegation.is_some()
            || plan.copy.is_some()
            || !self.static_constructor_body(call.constructor)?
        {
            return Ok(false);
        }
        let class = plan.class;
        // A primary base at offset zero is still the initialized root in
        // ConstantInterpreter::run_constructor. Other constructed subobjects
        // must have a literal type, including its trivial destructor.
        if at != 0 {
            if !self.graph.destructor_trivial(class)? || !plan.const_default {
                return Ok(false);
            }
            for &(member, _) in &plan.members {
                let mut ty = self.graph.lower_entity_type(member)?;
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                if !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                    && self.graph.types.get(ty).cv.0 & 2 != 0
                {
                    return Ok(false);
                }
            }
        }
        if let Some(function) = self.graph.lower_constructor_entity(call.constructor)
            && !self.graph.entity(function).constexpr
            && !matches!(
                self.graph.entity(function).special_definition,
                Some(super::special_members::SpecialDefinition::Defaulted { .. })
            )
        {
            return Ok(false);
        }
        self.class_layout(class, SourceOffset(0))?;
        if self.graph.lower_polymorphic(class) && !call.primary {
            out.push((at, 8, Item::Table(class)));
        }
        for index in 0..self.graph.constructors.get(call.constructor).bases.len() {
            let base = self.graph.constructors.get(call.constructor).bases[index];
            if base.forwards {
                return Ok(false);
            }
            let Some(constructor) = base.constructor else {
                return Ok(false);
            };
            let Some(arguments) = self.evaluated_arguments(base.initializer) else {
                return Ok(false);
            };
            let offset = self.graph.entity(class).bases[index]
                .offset
                .ok_or(Error::Invariant("evaluated base without layout"))?;
            let child = EvaluatedCall {
                constructor,
                zeroed: call.zeroed
                    || base.initializer.is_some()
                        && arguments.as_slice().is_empty()
                        && self.graph.lower_constructor(constructor).zero_before_value,
                primary: self.graph.lower_polymorphic(class)
                    && self.graph.lower_polymorphic(base.class)
                    && offset == 0,
                arguments,
            };
            if !self.evaluated_constructor(
                &child,
                (scope, at.wrapping_add(offset)),
                out,
                depth + 1,
            )? {
                return Ok(false);
            }
        }
        let mut next_subobject = 0;
        for index in 0..self.graph.constructors.get(call.constructor).members.len() {
            let (member, initializer) =
                self.graph.constructors.get(call.constructor).members[index];
            let sub = self
                .graph
                .constructors
                .get(call.constructor)
                .subobjects
                .get(next_subobject)
                .filter(|sub| sub.member == member)
                .copied();
            next_subobject += usize::from(sub.is_some());
            if self.graph.unnamed_bit_field(member) {
                continue;
            }
            if let Some(field) = self.graph.entity(member).bit_field {
                if initializer.is_some() || !call.zeroed {
                    return Ok(false);
                }
                let storage = self.graph.bit_fields.storage(field)?;
                if storage.shared || storage.split != 0 || storage.bytes > 8 {
                    out.unsupported();
                    continue;
                }
                out.push((
                    at.wrapping_add(storage.offset),
                    u64::from(storage.bytes),
                    Item::BitUnit(storage.bytes, 0),
                ));
                continue;
            }
            let offset = self
                .graph
                .entity(member)
                .member_offset
                .ok_or(Error::Invariant("evaluated member without layout"))?;
            let ty = self.graph.lower_entity_type(member)?;
            let origin = at.wrapping_add(offset);
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                if call.zeroed
                    && initializer.is_none()
                    && sub.is_some_and(|sub| self.graph.lower_constructor_trivial(sub.constructor))
                {
                    let mut image = Vec::new();
                    if !self.static_value_zero(ty, &mut image)? {
                        return Ok(false);
                    }
                    self.place_static_items(origin, &image, out)?;
                    continue;
                }
                let Some(sub) = sub else {
                    return Ok(false);
                };
                let Some(arguments) = self.evaluated_arguments(initializer) else {
                    return Ok(false);
                };
                let child = EvaluatedCall {
                    constructor: sub.constructor,
                    primary: false,
                    zeroed: call.zeroed
                        || initializer.is_some()
                            && arguments.as_slice().is_empty()
                            && self
                                .graph
                                .lower_constructor(sub.constructor)
                                .zero_before_value,
                    arguments,
                };
                if !self.evaluated_constructor(&child, (scope, origin), out, depth + 1)? {
                    return Ok(false);
                }
            } else if !self.evaluated_scalar_member(call, ty, initializer, (scope, origin), out)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn evaluated_scalar_member(
        &mut self,
        call: &EvaluatedCall,
        ty: TypeId,
        initializer: Option<ast::Initializer>,
        (scope, at): (ScopeId, u64),
        out: &mut Placements,
    ) -> Result<bool, Error> {
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            let Some(expr) = initializer.and_then(|i| self.single_initializer(i)) else {
                return Ok(false);
            };
            let mut image = Vec::new();
            if !self.static_array_element(ty, Some(expr), scope, &mut image)? {
                return Ok(false);
            }
            self.place_static_items(at, &image, out)?;
            return Ok(true);
        }
        if matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. }) {
            let mut image = Vec::new();
            let complete = if let Some(initializer) = initializer {
                let (ast::InitializerKind::Braced(expr) | ast::InitializerKind::Assign(expr)) =
                    initializer.kind
                else {
                    return Ok(false);
                };
                self.static_array_element(ty, Some(expr), scope, &mut image)?
            } else {
                call.zeroed && self.static_value_zero(ty, &mut image)?
            };
            if complete {
                self.place_static_items(at, &image, out)?;
            }
            return Ok(complete);
        }
        let value = if let Some(initializer) = initializer {
            if let Some(expr) = self.single_initializer(initializer) {
                match self.static_parameter_value(
                    call.constructor,
                    call.arguments.as_slice(),
                    expr,
                    ty,
                    scope,
                )? {
                    Some(value) => Some(value),
                    None => self.static_scalar(expr, ty, scope)?,
                }
            } else if self
                .evaluated_arguments(Some(initializer))
                .is_some_and(|arguments| arguments.as_slice().is_empty())
            {
                self.evaluated_zero(ty)
            } else {
                None
            }
        } else if call.zeroed {
            self.evaluated_zero(ty)
        } else {
            None
        };
        let Some(value) = value else {
            return Ok(false);
        };
        if !matches!(value, Value::Integer(0))
            || !matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_))
        {
            out.push((
                at,
                self.type_size(ty, SourceOffset(0))?.0,
                Item::Scalar(ty, value),
            ));
        }
        Ok(true)
    }
    fn evaluated_zero(&self, ty: TypeId) -> Option<Value> {
        match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Basic(f @ (F::Float | F::Double | F::LongDouble))) => {
                Some(Value::Floating(f, 0))
            }
            TypeKind::MemberPointer { .. } => Some(Value::MemberPointer(
                super::symbolic_members::MemberPointer::null(ty),
            )),
            TypeKind::Scalar(_) | TypeKind::Enum(_) | TypeKind::Pointer(_) => {
                Some(Value::Integer(0))
            }
            _ => None,
        }
    }
    fn place_static_items(
        &mut self,
        mut at: u64,
        image: &[Item],
        out: &mut Placements,
    ) -> Result<(), Error> {
        for &item in image {
            let bytes = match item {
                Item::Zero(bytes) => bytes,
                Item::Scalar(ty, _) => self.type_size(ty, SourceOffset(0))?.0,
                Item::BitUnit(bytes, _) => u64::from(bytes),
                Item::Table(_) => 8,
            };
            if !matches!(item, Item::Zero(_)) {
                out.push((at, bytes, item));
            }
            at = at
                .checked_add(bytes)
                .ok_or(Error::Limit("evaluated image placement"))?;
        }
        Ok(())
    }
    pub(super) fn static_zero_prvalue(
        &mut self,
        target: TypeId,
        expr: ast::ExprId,
        out: &mut Vec<Item>,
    ) -> Result<Option<bool>, Error> {
        self.check_stack()?;
        let root = self.graph.expression_facts.root(expr);
        let facts = self.graph.expression_facts.get(root)?;
        if let Some(constructor) = facts.constructor {
            if matches!(
                facts.class_initialization,
                Some(ClassInitialization::Constructed(
                    ConstructorArguments::Value
                ))
            ) && !self.graph.constructors.get(constructor).nontrivial()
            {
                return self.static_value_zero(target, out).map(Some);
            }
            return Ok(None);
        }
        if let Some(ClassInitialization::FromExpression(source)) = facts.class_initialization {
            return self.static_zero_prvalue(target, source, out);
        }
        Ok(None)
    }
    pub(super) fn static_value_zero(
        &mut self,
        target: TypeId,
        out: &mut Vec<Item>,
    ) -> Result<bool, Error> {
        // Evaluated objects place base/member values at their actual offsets.
        // Base tail padding and empty bases do not occupy a second span.
        let size = self.type_size(target, SourceOffset(0))?.0;
        let mut placements = Vec::new();
        if !self.static_zero_placements(target, 0, &mut placements)? {
            return Ok(false);
        }
        placements.sort_by_key(|&(offset, _, _)| offset);
        let mut position = 0;
        for (offset, size, item) in placements {
            if offset < position {
                if matches!(item, Item::BitUnit(..)) {
                    continue;
                }
                return Ok(false);
            }
            append_zero(out, offset - position)?;
            out.push(item);
            position = offset
                .checked_add(size)
                .ok_or(Error::Limit("static zero span"))?;
        }
        append_zero(
            out,
            size.checked_sub(position)
                .ok_or(Error::Invariant("static zero object span"))?,
        )?;
        Ok(true)
    }
    fn static_zero_placements(
        &mut self,
        target: TypeId,
        origin: u64,
        out: &mut Vec<(u64, u64, Item)>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        match self.graph.types.get(target).kind {
            TypeKind::Class(class) => {
                for index in 0..self.graph.entity(class).bases.len() {
                    let base = &self.graph.entity(class).bases[index];
                    let offset = base
                        .offset
                        .ok_or(Error::Invariant("static zero base without offset"))?;
                    let ty = self.graph.lower_entity_type(base.class)?;
                    let at = origin
                        .checked_add(offset)
                        .ok_or(Error::Limit("static zero base span"))?;
                    if !self.static_zero_placements(ty, at, out)? {
                        return Ok(false);
                    }
                }
                let union = self.graph.entity(class).key == crate::lex::vocabulary::Keyword::Union;
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    if self.graph.unnamed_bit_field(member) {
                        continue;
                    }
                    let ty = self.graph.lower_entity_type(member)?;
                    if let Some(field) = self.graph.entity(member).bit_field {
                        let storage = self.graph.bit_fields.storage(field)?;
                        if storage.shared || storage.split != 0 || storage.bytes > 8 {
                            return Ok(false);
                        }
                        let at = origin
                            .checked_add(storage.offset)
                            .ok_or(Error::Limit("static zero bit unit"))?;
                        out.push((
                            at,
                            u64::from(storage.bytes),
                            Item::BitUnit(storage.bytes, 0),
                        ));
                    } else {
                        let offset = self
                            .graph
                            .entity(member)
                            .member_offset
                            .ok_or(Error::Invariant("static zero member without offset"))?;
                        let at = origin
                            .checked_add(offset)
                            .ok_or(Error::Limit("static zero member span"))?;
                        if !self.static_zero_placements(ty, at, out)? {
                            return Ok(false);
                        }
                    }
                    if union {
                        break;
                    }
                }
            }
            TypeKind::Array { element, bound, .. } => {
                let size = self.type_size(element, SourceOffset(0))?.0;
                for index in 0..bound {
                    let at = index
                        .checked_mul(size)
                        .and_then(|offset| origin.checked_add(offset))
                        .ok_or(Error::Limit("static zero array span"))?;
                    if !self.static_zero_placements(element, at, out)? {
                        return Ok(false);
                    }
                }
            }
            TypeKind::MemberPointer { .. } => out.push((
                origin,
                self.type_size(target, SourceOffset(0))?.0,
                Item::Scalar(
                    target,
                    Value::MemberPointer(super::symbolic_members::MemberPointer::null(
                        self.bare_type(target)?,
                    )),
                ),
            )),
            TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr) => {}
            TypeKind::Scalar(Scalar::Basic(ty @ (F::Float | F::Double | F::LongDouble))) => {
                out.push((
                    origin,
                    self.type_size(target, SourceOffset(0))?.0,
                    Item::Scalar(target, Value::Floating(ty, 0)),
                ));
            }
            TypeKind::Scalar(_) | TypeKind::Enum(_) => {
                out.push((
                    origin,
                    self.type_size(target, SourceOffset(0))?.0,
                    Item::Scalar(target, Value::Integer(0)),
                ));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
