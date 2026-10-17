// Translated from constant_interpreter.cpp and constant_construction.cpp. See NOTICE.
//! Flat evaluation storage for class and array skeletons, with canonical subobject keys.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constexpr_calls::Returned,
    constexpr_frames::CellId,
    expression_facts::{ObjectSource, ValueCategory},
    graph::{EntityId, ScopeId},
    inheritance::BasePathId,
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

impl Analyzer<'_> {
    pub(super) fn allocate_constant_object(
        &mut self,
        ty: TypeId,
        value: Option<Value>,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let ty = self.bare_type(ty)?;
        self.constant_object_leaves(ty, 0, location)?;
        let place = self.constexpr_frames.allocate(ty, value, location)?;
        let root = self.constexpr_frames.locate(place, location)?;
        self.constant_skeleton(place, root, location)?;
        Ok(place)
    }
    fn constant_object_leaves(
        &mut self,
        ty: TypeId,
        depth: u32,
        location: SourceOffset,
    ) -> Result<u64, Error> {
        self.check_stack()?;
        let count = match self.graph.types.get(ty).kind {
            TypeKind::Array { element, bound, .. } => bound
                .max(1)
                .saturating_mul(self.constant_object_leaves(element, depth, location)?),
            TypeKind::Class(class) if depth <= 64 => {
                self.complete_class(class)?;
                if self.graph.lower_nontrivial_destructor(class)? {
                    return Err(nonconstant(
                        location,
                        "an object with a non-trivial destructor in a constant expression",
                    ));
                }
                let mut count = 1u64;
                for index in 0..self.graph.entity(class).bases.len() {
                    let child = self.graph.entity(class).bases[index].class;
                    let ty = self.graph.lower_entity_type(child)?;
                    count = count.saturating_add(self.constant_object_leaves(
                        ty,
                        depth + 1,
                        location,
                    )?);
                }
                for index in 0..self.graph.entity(class).members.len() {
                    let child = self.graph.entity(class).members[index];
                    let ty = self.graph.lower_entity_type(child)?;
                    count = count.saturating_add(self.constant_object_leaves(
                        ty,
                        depth + 1,
                        location,
                    )?);
                }
                count
            }
            _ => 1,
        };
        if count > 1 << 22 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its storage budget",
            ));
        }
        Ok(count)
    }
    fn constant_skeleton(
        &mut self,
        place: Address,
        cell: CellId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.types.get(place.designated).kind {
            TypeKind::Array { element, bound, .. } => {
                let size = self.type_size(element, location)?.0;
                for index in 0..bound {
                    let ty = self.bare_type(element)?;
                    let child = Address {
                        designated: ty,
                        offset: place
                            .offset
                            .wrapping_add(super::integral::signed(index.wrapping_mul(size))),
                        ..place
                    };
                    let cell = self.constexpr_frames.add_cell(child, None, location)?;
                    self.constant_skeleton(child, cell, location)?;
                }
            }
            TypeKind::Class(class) => {
                self.class_layout(class, location)?;
                if self.graph.entity(class).key == Keyword::Union {
                    return Err(Error::Missing("constexpr union active member storage"));
                }
                for index in 0..self.graph.entity(class).bases.len() {
                    let base = self.graph.entity(class).bases[index];
                    let ty = self.bare_type(self.graph.lower_entity_type(base.class)?)?;
                    let offset = base
                        .offset
                        .ok_or(Error::Invariant("constant base without layout"))?;
                    let child = Address {
                        designated: ty,
                        offset: place.offset.wrapping_add(super::integral::signed(offset)),
                        ..place
                    };
                    let cell = self.constexpr_frames.add_cell(child, None, location)?;
                    self.constant_skeleton(child, cell, location)?;
                }
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    if self.graph.unnamed_bit_field(member) {
                        continue;
                    }
                    let child = self.constant_member_slot(place, member, location)?;
                    let cell = self.constexpr_frames.add_cell(child, None, location)?;
                    self.constant_skeleton(child, cell, location)?;
                }
            }
            _ => (),
        }
        self.constexpr_frames.finish_cell(cell);
        Ok(())
    }
    pub(super) fn constant_member_slot(
        &mut self,
        place: Address,
        member: EntityId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let entity = self.graph.entity(member);
        let offset = if let Some(field) = entity.bit_field {
            self.graph.bit_fields.storage(field)?.bit / 8
        } else {
            entity
                .member_offset
                .ok_or(Error::Invariant("constant member without layout"))?
        };
        let field = entity.bit_field.map(|_| member);
        let ty = self.bare_type(self.graph.lower_entity_type(member)?)?;
        let _ = location;
        Ok(Address {
            designated: ty,
            offset: place.offset.wrapping_add(super::integral::signed(offset)),
            bit_field: field,
            ..place
        })
    }
    pub(super) fn constant_member_designation(
        &mut self,
        object: Address,
        member: EntityId,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let mut place = self.constant_member_slot(object, member, location)?;
        if matches!(
            self.graph.types.get(place.designated).kind,
            TypeKind::Reference { .. }
        ) {
            let Value::Pointer(referent) = self.constexpr_frames.read(place, location)? else {
                return Err(Error::Invariant(
                    "constant reference member without binding",
                ));
            };
            place = referent;
        }
        place.designated = ty;
        Ok(place)
    }
    pub(super) fn store_constant_value(
        &mut self,
        mut place: Address,
        mut value: Value,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Some(member) = place.bit_field
            && let Value::Integer(integer) = value
        {
            let field = self
                .graph
                .entity(member)
                .bit_field
                .ok_or(Error::Invariant("constant bit field without identity"))?;
            let width = self.graph.bit_fields.width(field);
            let kind = self
                .integral_kind(place.designated)
                .ok_or(Error::Invariant("nonintegral constant bit field"))?;
            if width < 64
                && !matches!(
                    self.graph.types.get(place.designated).kind,
                    TypeKind::Scalar(super::types::Scalar::Bool)
                )
            {
                let mask = (1u64 << width) - 1;
                let mut bits = integer.bits & mask;
                if kind.signed && width != 0 && bits >> (width - 1) != 0 {
                    bits |= !mask;
                }
                value = Value::Integer(super::constants::Constant {
                    bits: kind.convert(bits),
                    ty: integer.ty,
                });
            }
        }
        place.designated = self.bare_type(place.designated)?;
        self.constexpr_frames.write(place, value, location)
    }
    pub(super) fn constant_zero_object(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        place.designated = self.bare_type(place.designated)?;
        let cells = self.constexpr_frames.cells(place, location)?;
        for index in cells {
            let child = self.constexpr_frames.cell_place(place, index);
            if matches!(
                self.graph.types.get(child.designated).kind,
                TypeKind::Class(_) | TypeKind::Array { .. }
            ) {
                continue;
            }
            if matches!(
                self.graph.types.get(child.designated).kind,
                TypeKind::Reference { .. }
            ) {
                continue;
            }
            let value = self.zero_constant_scalar(child.designated)?;
            self.store_constant_value(child, value, location)?;
        }
        Ok(())
    }
    pub(super) fn constant_object_place(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        let id = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(id)?;
        if facts.category != ValueCategory::Prvalue {
            return self.constant_place(id, scope);
        }
        let location = self.tree.expressions.get(id).location;
        let place = self.allocate_constant_object(facts.ty, None, location)?;
        self.materialize_constant_object(id, place, scope)?;
        Ok(place)
    }
    pub(super) fn constant_receiver_place(
        &mut self,
        source: ObjectSource,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        match source {
            ObjectSource::Implicit => self
                .constexpr_frames
                .current()
                .ok()
                .and_then(|frame| frame.object)
                .ok_or_else(|| {
                    nonconstant(location, "`this` outside a constexpr member function call")
                }),
            ObjectSource::Explicit {
                expression,
                arrow: false,
            } => self.constant_object_place(expression, scope),
            ObjectSource::Explicit {
                expression,
                arrow: true,
            } => {
                let place = self.address_operand(expression, scope)?;
                if place.root == Root::Null {
                    return Err(nonconstant(
                        location,
                        "a member function called through a null pointer",
                    ));
                }
                Ok(place)
            }
        }
    }
    pub(super) fn constant_project_path(
        &mut self,
        mut place: Address,
        path: Option<BasePathId>,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if let Some(path) = path {
            let class = self
                .graph
                .class_of(place.designated)
                .ok_or(Error::Invariant("constant base path without class"))?;
            self.class_layout(class, location)?;
            for &(class, index) in self.graph.base_paths.get(path) {
                let edge = self.graph.entity(class).bases[index];
                place.offset = place.offset.wrapping_add(super::integral::signed(
                    edge.offset
                        .ok_or(Error::Invariant("constant base without offset"))?,
                ));
                place.designated = self.graph.lower_entity_type(edge.class)?;
            }
        }
        Ok(place)
    }
    pub(super) fn constant_conversion_result(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Returned, Error> {
        let location = self.tree.expressions.get(id).location;
        let step = self
            .graph
            .expression_facts
            .function_conversion(id)
            .ok_or(Error::Invariant("constant conversion without selection"))?;
        let function = self.graph.binding(step.binding).entity;
        let place = self.constant_object_place(id, scope)?;
        let place = self.constant_project_path(place, step.path, location)?;
        if self.graph.virtuals.function(function).is_virtual {
            return Err(nonconstant(
                location,
                "a virtual conversion function in a constant expression",
            ));
        }
        self.invoke_constant_function(function, &[], Some(place), scope, location)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{analyzer::Analyzer, constants::ConstantState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn objects_keep_invocation_values_local_and_reuse_concrete_member_bodies() {
        let source = SourceBuffer::new(
            br"
            template<class T>struct S{
                T n;
                constexpr S(T x):n(x){}
                constexpr operator T()const {int sum=0;for(int i=0;i<n;++i)sum+=i;return sum;}
                void unused(){T::missing;}
            };
            constexpr int a=int(S<int>(3));
            constexpr int b=int(S<int>(5));
            constexpr int f(){S<int> s(4);const S<int>& r=s;return int(r);}
            constexpr int c=f();
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constexpr-objects").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a".as_slice(), b"b", b"c"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        for (name, bits) in names.into_iter().zip([3, 10, 6]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(analyzer.graph.entity(analyzer.graph.binding(binding).entity).constant,
                ConstantState::Value(value) if value.bits == bits)
            );
        }
        let declaration = analyzer.tree.decl_lists.get(declarations)[1];
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let mark = analyzer.tree.expressions.mark();
        assert_eq!(
            analyzer
                .constant_value(expr, global, true)
                .unwrap()
                .unwrap()
                .bits,
            3
        );
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
    }
}
