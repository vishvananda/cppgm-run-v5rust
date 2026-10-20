// Translated from lower/global_lowering.cpp::static_construction/static_constructor. See NOTICE.
//! Analysis-owned static placements and ordered demand from a bounded construction attempt.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    graph::{EntityId, Graph, ScopeId},
    static_arrays::{Item, append_zero},
    static_initialization::Value,
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Images {
    attempts: HashMap<EntityId, Attempt>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{layout::LayoutState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn images_keep_evaluation_and_promotion_demand_distinct_without_reparsing() {
        let source = SourceBuffer::new(
            br"
            struct Empty{virtual ~Empty(){}}; Empty empty;
            struct Derived:Empty{}; Derived derived;
            int target; int dynamic();
            struct Promoted{int*p;int n;Promoted():p(&target),n(3){}virtual ~Promoted(){}};
            Promoted promoted;
            struct Partial{int*p;int n;Partial():p(&target),n(dynamic()){}virtual ~Partial(){}};
            Partial partial;
            #pragma pack(push,1)
            struct Packed{short low:3;int wide:30;virtual int f(){return wide;}};
            #pragma pack(pop)
            Packed packed{};
            Empty over_budget[4194305]{};
            struct VirtualRoot{virtual int f(){return 1;}};
            struct VirtualChild:virtual VirtualRoot{}; VirtualChild virtual_child;
            struct IndirectChild:VirtualChild{}; IndirectChild indirect_child;
            struct Unused{virtual void f();};
            void local(){Empty automatic;}
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"static-images").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let mut entity = |name: &[u8]| {
            graph
                .binding(
                    graph
                        .own(
                            graph.global,
                            cursor.intern_name(name).unwrap(),
                            Filter::Ordinary,
                        )
                        .unwrap(),
                )
                .entity
        };
        let empty = entity(b"empty");
        let derived = entity(b"derived");
        let promoted = entity(b"promoted");
        let partial = entity(b"partial");
        let base = graph
            .constructors
            .get(graph.lower_construction(empty).unwrap())
            .class;
        let child = graph
            .constructors
            .get(graph.lower_construction(derived).unwrap())
            .class;
        assert!(graph.lower_evaluated_construction(empty));
        assert!(graph.lower_evaluated_construction(derived));
        assert!(
            matches!(graph.lower_static_construction(empty), Some([Item::Table(c)]) if *c == base)
        );
        assert!(
            matches!(graph.lower_static_construction(derived), Some([Item::Table(c)]) if *c == child)
        );
        assert!(!graph.lower_evaluated_construction(promoted));
        assert!(graph.lower_static_construction(promoted).is_some());
        assert!(graph.lower_static_construction(partial).is_none());
        assert!(matches!(graph.lower_static_construction_demands(partial),
            [Item::Table(_), Item::Scalar(_, Value::Address(root, 0))] if *root == entity(b"target")));
        assert!(graph.lower_static_construction(entity(b"packed")).is_none());
        assert!(matches!(
            graph.lower_static_construction_demands(entity(b"packed")),
            [Item::Table(_)]
        ));
        assert!(
            graph
                .lower_static_construction(entity(b"over_budget"))
                .is_none()
        );
        for name in [b"virtual_child".as_slice(), b"indirect_child"] {
            let child = entity(name);
            assert!(!graph.lower_evaluated_construction(child));
            assert!(graph.lower_static_construction(child).is_none());
            assert!(graph.lower_static_construction_demands(child).is_empty());
        }
        assert_eq!(graph.static_constructions.attempts.len(), 5);
        let unused = graph
            .binding(
                graph
                    .own(
                        graph.global,
                        cursor.intern_name(b"Unused").unwrap(),
                        Filter::Prefix,
                    )
                    .unwrap(),
            )
            .entity;
        assert!(matches!(
            graph.entity(unused).layout,
            LayoutState::NotStarted
        ));
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
    }
}
#[derive(Default)]
struct Attempt {
    demands: Vec<Item>,
    image: Option<Vec<Item>>,
    evaluated: bool,
}
#[derive(Clone, Copy)]
struct Placement {
    offset: u64,
    size: u64,
    item: Item,
}
#[derive(Default)]
struct Construction {
    placements: Vec<Placement>,
    demands: Vec<Item>,
}
impl Graph {
    pub(crate) fn lower_static_construction(&self, entity: EntityId) -> Option<&[Item]> {
        self.static_constructions
            .attempts
            .get(&entity)?
            .image
            .as_deref()
    }
    pub(crate) fn lower_evaluated_construction(&self, entity: EntityId) -> bool {
        self.aggregate_constants.get(entity).is_some()
            || self
                .static_constructions
                .attempts
                .get(&entity)
                .is_some_and(|a| a.evaluated)
    }
    pub(crate) fn lower_static_construction_demands(&self, entity: EntityId) -> &[Item] {
        self.static_constructions
            .attempts
            .get(&entity)
            .map_or(&[], |attempt| attempt.demands.as_slice())
    }
}
impl Construction {
    fn tables(&mut self, class: EntityId, origin: u64, first: usize, cursor: &mut u64) -> bool {
        let item = Item::Table(class);
        self.demands.push(item);
        if *cursor == 0 {
            self.placements.push(Placement {
                offset: origin,
                size: 8,
                item,
            });
        } else {
            let Some(prior) = self.placements[first..].iter_mut().rev().find(|p| {
                p.offset == origin
                    && matches!(
                        p.item,
                        Item::Table(_)
                            | Item::Scalar(
                                _,
                                Value::Address(..) | Value::Literal(..) | Value::Temporary(..)
                            )
                    )
            }) else {
                return false;
            };
            prior.item = item;
        }
        *cursor = (*cursor).max(8);
        true
    }
    fn image(&mut self, size: u64) -> Result<Option<Vec<Item>>, Error> {
        self.placements.sort_by_key(|p| p.offset);
        let mut out = Vec::with_capacity(self.placements.len());
        let mut cursor = 0;
        for &placement in &self.placements {
            if placement.offset < cursor {
                return Ok(None);
            }
            append_zero(&mut out, placement.offset - cursor)?;
            out.push(placement.item);
            cursor = placement
                .offset
                .checked_add(placement.size)
                .ok_or(Error::Limit("static construction image"))?;
        }
        if cursor > size {
            return Ok(None);
        }
        append_zero(&mut out, size - cursor)?;
        Ok(Some(out))
    }
}
impl Analyzer<'_> {
    pub(super) fn record_static_construction(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self
            .graph
            .static_constructions
            .attempts
            .contains_key(&entity)
        {
            return Ok(());
        }
        let facts = self.graph.lower_variable(entity)?;
        let mut object = facts.ty;
        while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
            object = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(object).kind else {
            return Ok(());
        };
        if facts.thread_local
            || self.graph.lower_automatic(entity)
            || !self.graph.lower_polymorphic(class)
            || self.graph.lower_array_image(entity).is_some()
        {
            return Ok(());
        }
        let mut prefix = Vec::new();
        if let Some(evaluated) = self.evaluated_polymorphic_object(entity, initializer, scope)? {
            prefix = evaluated.demands;
            let complete = evaluated.image.is_some();
            self.graph.static_constructions.attempts.insert(
                entity,
                Attempt {
                    demands: prefix.clone(),
                    image: evaluated.image,
                    evaluated: complete,
                },
            );
            if complete {
                return Ok(());
            }
        }
        // static_construction rejects a class with virtual bases before
        // attempting any constructor or demanding its subobjects' tables.
        if object != facts.ty || self.graph.lower_virtual_class(class)?.has_virtual_bases {
            return Ok(());
        }
        let arguments = match initializer.map(|i| i.kind) {
            None => &[][..],
            Some(ast::InitializerKind::Parenthesized(values)) => self.tree.expr_lists.get(values),
            _ => return Ok(()),
        };
        let constructor = if let Some(constructor) = self.graph.lower_construction(entity) {
            constructor
        } else if let Some(expr) = initializer.and_then(|i| self.single_initializer(i)) {
            let Some(super::copy_initialization::CopyAction::Constructor { constructor, .. }) =
                self.graph.lower_class_copy(expr)?
            else {
                return Ok(());
            };
            constructor
        } else {
            return Ok(());
        };
        let mut construction = Construction {
            demands: prefix,
            ..Construction::default()
        };
        let argument_buffer = super::call_arguments::Arguments::new(arguments);
        let end = self.static_constructor(
            constructor,
            argument_buffer.as_slice(),
            scope,
            0,
            &mut construction,
            0,
        )?;
        let size = self.class_layout(class, SourceOffset(0))?.0;
        let image = if end.is_some_and(|end| end <= size) {
            construction.image(size)?
        } else {
            None
        };
        self.graph.static_constructions.attempts.insert(
            entity,
            Attempt {
                demands: construction.demands,
                image,
                evaluated: false,
            },
        );
        Ok(())
    }
    fn static_constructor(
        &mut self,
        constructor: ConstructorId,
        arguments: &[ast::ExprId],
        scope: ScopeId,
        origin: u64,
        out: &mut Construction,
        depth: u32,
    ) -> Result<Option<u64>, Error> {
        self.check_stack()?;
        let plan = self.graph.constructors.get(constructor);
        if depth > 64 || plan.delegation.is_some() || !self.static_constructor_body(constructor)? {
            return Ok(None);
        }
        let class = plan.class;
        let function = plan
            .binding
            .map(|binding| self.graph.binding(binding).entity);
        if let Some(function) = function {
            let ty = self.graph.lower_entity_type(function)?;
            if !matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Function {
                    variadic: false,
                    ..
                }
            ) || arguments.len() > self.graph.types.function_parameter_count(ty)?
            {
                return Ok(None);
            }
        }
        self.class_layout(class, SourceOffset(0))?;
        let first = out.placements.len();
        let mut cursor = 0;
        let mut tables = false;
        let mut next_subobject = 0;
        for index in 0..self.graph.constructors.get(constructor).bases.len() {
            let base = self.graph.constructors.get(constructor).bases[index];
            let offset = self.graph.entity(class).bases[index]
                .offset
                .ok_or(Error::Invariant("static base without layout"))?;
            if base.forwards || offset != cursor {
                return Ok(None);
            }
            let ty = self.graph.lower_entity_type(base.class)?;
            let Some(_) = self.static_class_subobject(
                base.constructor,
                ty,
                base.initializer,
                (scope, origin.wrapping_add(offset)),
                out,
                depth,
            )?
            else {
                return Ok(None);
            };
            cursor = offset.wrapping_add(self.type_size(ty, SourceOffset(0))?.0);
        }
        for index in 0..self.graph.constructors.get(constructor).members.len() {
            let (member, initializer) = self.graph.constructors.get(constructor).members[index];
            let sub = self
                .graph
                .constructors
                .get(constructor)
                .subobjects
                .get(next_subobject)
                .filter(|sub| sub.member == member)
                .copied();
            next_subobject += usize::from(sub.is_some());
            let ty = self.graph.lower_entity_type(member)?;
            if self.graph.entity(member).bit_field.is_some()
                || matches!(self.graph.types.get(ty).kind, TypeKind::Class(child)
                    if self.graph.anonymous_member_storage(child).is_some())
            {
                return Ok(None);
            }
            if !tables && self.graph.lower_polymorphic(class) {
                if !out.tables(class, origin, first, &mut cursor) {
                    return Ok(None);
                }
                tables = true;
            }
            let offset = self
                .graph
                .entity(member)
                .member_offset
                .ok_or(Error::Invariant("static member without layout"))?;
            if offset < cursor {
                return Ok(None);
            }
            let at = origin.wrapping_add(offset);
            let end = if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                let written = initializer
                    .filter(|_| self.graph.constructors.get(constructor).written_members[index]);
                self.static_class_subobject(
                    sub.map(|s| s.constructor),
                    ty,
                    written,
                    (scope, at),
                    out,
                    depth,
                )?
            } else {
                self.static_scalar_subobject(
                    constructor,
                    arguments,
                    ty,
                    initializer,
                    (scope, at),
                    out,
                )?
            };
            let Some(end) = end else {
                return Ok(None);
            };
            cursor = offset.wrapping_add(end.max(self.type_size(ty, SourceOffset(0))?.0));
        }
        if !tables
            && self.graph.lower_polymorphic(class)
            && !out.tables(class, origin, first, &mut cursor)
        {
            return Ok(None);
        }
        Ok(Some(cursor))
    }
    pub(super) fn static_constructor_body(
        &self,
        constructor: ConstructorId,
    ) -> Result<bool, Error> {
        let Some(entity) = self.graph.lower_constructor_entity(constructor) else {
            return Ok(true);
        };
        if matches!(
            self.graph.entity(entity).copy_origin,
            super::copy_construction::Origin::Implicit
        ) {
            return Ok(true);
        }
        if let Some(super::special_members::SpecialDefinition::Defaulted { first, .. }) =
            self.graph.entity(entity).special_definition
        {
            return Ok(first);
        }
        let Some(body) = self.graph.lower_function_body(entity) else {
            return Ok(false);
        };
        let ast::Body::Parsed(body) = self.tree.bodies.get(body) else {
            return Err(Error::Invariant("static constructor with unparsed body"));
        };
        Ok(body.handlers.is_none()
            && matches!(self.tree.statements.get(body.compound).kind,
            ast::StmtKind::Compound(items) if self.tree.stmt_lists.get(items).is_empty()))
    }
    fn static_class_subobject(
        &mut self,
        constructor: Option<ConstructorId>,
        ty: TypeId,
        initializer: Option<ast::Initializer>,
        (scope, at): (ScopeId, u64),
        out: &mut Construction,
        depth: u32,
    ) -> Result<Option<u64>, Error> {
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("static class subobject type"));
        };
        if constructor.is_none_or(|id| self.graph.lower_constructor_trivial(id)) {
            return Ok(initializer.is_none().then_some(0));
        }
        if self.graph.entity(class).key == Keyword::Union {
            return Ok(None);
        }
        let arguments = match initializer.map(|i| i.kind) {
            None => &[][..],
            Some(ast::InitializerKind::Parenthesized(values)) => self.tree.expr_lists.get(values),
            _ => return Ok(None),
        };
        let argument_buffer = super::call_arguments::Arguments::new(arguments);
        self.static_constructor(
            constructor.ok_or(Error::Invariant("static constructor"))?,
            argument_buffer.as_slice(),
            scope,
            at,
            out,
            depth + 1,
        )
    }
    fn static_scalar_subobject(
        &mut self,
        constructor: ConstructorId,
        arguments: &[ast::ExprId],
        ty: TypeId,
        initializer: Option<ast::Initializer>,
        (scope, at): (ScopeId, u64),
        out: &mut Construction,
    ) -> Result<Option<u64>, Error> {
        let mut element = ty;
        while let TypeKind::Array { element: inner, .. } = self.graph.types.get(element).kind {
            element = inner;
        }
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            || element != ty
                && (initializer.is_some()
                    || matches!(self.graph.types.get(element).kind, TypeKind::Class(_)))
        {
            return Ok(None);
        }
        let Some(initializer) = initializer else {
            return Ok(Some(0));
        };
        let Some(expr) = self.single_initializer(initializer) else {
            return Ok(matches!(initializer.kind, ast::InitializerKind::Braced(id)
                if matches!(self.tree.expressions.get(id).kind, ast::ExprKind::Braced { elements, .. }
                    if self.tree.expr_lists.get(elements).is_empty())).then_some(0));
        };
        let value = match self.static_parameter_value(constructor, arguments, expr, ty, scope)? {
            Some(value) => Some(value),
            None => self.static_scalar(expr, ty, scope)?,
        };
        let Some(value) = value else {
            return Ok(None);
        };
        if matches!(value, Value::Integer(0))
            && matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_))
        {
            return Ok(Some(0));
        }
        let item = Item::Scalar(ty, value);
        out.demands.push(item);
        out.placements.push(Placement {
            offset: at,
            size: self.type_size(ty, SourceOffset(0))?.0,
            item,
        });
        Ok(Some(0))
    }
    pub(super) fn static_parameter_value(
        &mut self,
        constructor: ConstructorId,
        arguments: &[ast::ExprId],
        mut expr: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<Option<Value>, Error> {
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
            expr = inner;
        }
        let ast::ExprKind::Id(name) = self.tree.expressions.get(expr).kind else {
            return Ok(None);
        };
        let Some(binding) = self.graph.resolved_name(name) else {
            return Ok(None);
        };
        let Some(function) = self.graph.lower_constructor_entity(constructor) else {
            return Ok(None);
        };
        let Some((index, (_, _, parameter))) = self
            .graph
            .lower_parameters(self.graph.entity(function).scope)
            .enumerate()
            .find(|(_, (id, _, _))| *id == self.graph.binding(binding).entity)
        else {
            return Ok(None);
        };
        if matches!(
            self.graph.types.get(parameter).kind,
            TypeKind::Reference { .. }
        ) {
            return Ok(None);
        }
        let argument = arguments.get(index).copied().or_else(|| {
            self.graph
                .entity(function)
                .defaults
                .get(index)
                .copied()
                .flatten()
                .map(|d| d.expression)
        });
        let Some(argument) = argument else {
            return Ok(None);
        };
        let Some(value) = self.static_scalar(argument, parameter, scope)? else {
            return Ok(None);
        };
        if self.bare_type(parameter)? == self.bare_type(target)? {
            return Ok(Some(value));
        }
        if matches!(value, Value::Integer(_))
            && self.integral_kind(parameter).is_some()
            && self.integral_kind(target).is_some()
        {
            return self.static_conversion(value, parameter, target);
        }
        Ok(None)
    }
}
