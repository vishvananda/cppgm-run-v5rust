// Translated from sema/constant_frames.h and constant_interpreter.cpp. See NOTICE.
//! Evaluation-owned objects and call frames. Reused slots carry generations.
use super::{
    Error,
    constant_addresses::{Address, Root, TemporaryValue, nonconstant},
    graph::{EntityId, ScopeId},
    types::TypeId,
};
use crate::support::source::SourceOffset;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ObjectId {
    slot: u32,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct CellId(u32);
#[derive(Clone, Copy)]
struct Cell {
    ty: TypeId,
    offset: i64,
    field: Option<EntityId>,
    value: Option<TemporaryValue>,
    end: usize,
}
struct Object {
    id: ObjectId,
    ty: TypeId,
    cells: std::ops::Range<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key(ObjectId, i64, TypeId, Option<EntityId>);
pub(super) struct Frame {
    pub result: TypeId,
    pub scope: ScopeId,
    pub object: Option<Address>,
    seeking: Option<crate::syntax::ast::StmtId>,
    places: HashMap<EntityId, Address>,
    locals: Vec<EntityId>,
}
impl Frame {
    pub(super) fn new(result: TypeId, scope: ScopeId) -> Self {
        Self {
            result,
            scope,
            seeking: None,
            object: None,
            places: HashMap::new(),
            locals: Vec::new(),
        }
    }
    pub(super) fn bind(&mut self, entity: EntityId, place: Address) {
        self.places.insert(entity, place);
        self.locals.push(entity);
    }
}
#[derive(Clone, Copy)]
pub(super) struct Region {
    objects: usize,
    cells: usize,
    locals: usize,
}
#[derive(Default)]
pub(super) struct Frames {
    objects: Vec<Object>,
    cells: Vec<Cell>,
    index: HashMap<Key, CellId>,
    stack: Vec<Frame>,
    generation: u64,
    steps: u64,
}
impl Frames {
    pub(super) fn seeking(&self) -> Option<crate::syntax::ast::StmtId> {
        self.stack.last()?.seeking
    }
    pub(super) fn seek(&mut self, id: Option<crate::syntax::ast::StmtId>) {
        if let Some(frame) = self.stack.last_mut() {
            frame.seeking = id;
        }
    }
    pub(super) fn active(&self) -> bool {
        !self.stack.is_empty()
    }
    pub(super) fn depth(&self) -> usize {
        self.stack.len()
    }
    pub(super) fn current(&self) -> Result<&Frame, Error> {
        self.stack
            .last()
            .ok_or(Error::Invariant("constant body without frame"))
    }
    pub(super) fn bind(&mut self, entity: EntityId, place: Address) -> Result<(), Error> {
        self.stack
            .last_mut()
            .ok_or(Error::Invariant("constant local without frame"))?
            .bind(entity, place);
        Ok(())
    }
    pub(super) fn local(&self, entity: EntityId) -> Option<Address> {
        self.stack.last()?.places.get(&entity).copied()
    }
    pub(super) fn push(&mut self, frame: Frame) {
        self.stack.push(frame);
    }
    pub(super) fn pop(&mut self) {
        self.stack.pop();
    }
    pub(super) fn region(&self) -> Region {
        Region {
            objects: self.objects.len(),
            cells: self.cells.len(),
            locals: self.stack.last().map_or(0, |f| f.locals.len()),
        }
    }
    pub(super) fn release(&mut self, region: Region) {
        if let Some(frame) = self.stack.last_mut() {
            for entity in frame.locals.drain(region.locals..) {
                frame.places.remove(&entity);
            }
        }
        for slot in region.objects..self.objects.len() {
            let object = &self.objects[slot];
            let id = object.id;
            for cell in &self.cells[object.cells.clone()] {
                self.index
                    .remove(&Key(id, cell.offset, cell.ty, cell.field));
            }
        }
        self.objects.truncate(region.objects);
        self.cells.truncate(region.cells);
    }
    pub(super) fn clear(&mut self) {
        self.objects.clear();
        self.cells.clear();
        self.index.clear();
        self.stack.clear();
        self.steps = 0;
    }
    pub(super) fn step(&mut self, location: SourceOffset) -> Result<(), Error> {
        self.steps += 1;
        if self.steps > 1 << 24 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its step budget",
            ));
        }
        Ok(())
    }
    pub(super) fn allocate(
        &mut self,
        ty: TypeId,
        value: Option<TemporaryValue>,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(Error::Limit("constant object generation limit"))?;
        let id = ObjectId {
            slot: u32::try_from(self.objects.len())
                .map_err(|_| Error::Limit("constant object limit"))?,
            generation: self.generation,
        };
        self.objects.push(Object {
            id,
            ty,
            cells: self.cells.len()..self.cells.len(),
        });
        let place = Address::new(Root::Object(id), ty);
        self.add_cell(place, value, location)?;
        Ok(place)
    }
    fn object(&self, id: ObjectId, location: SourceOffset) -> Result<&Object, Error> {
        self.objects
            .get(id.slot as usize)
            .filter(|object| object.id == id)
            .ok_or_else(|| nonconstant(location, "an access to an object whose lifetime has ended"))
    }
    pub(super) fn ty(&self, id: ObjectId, location: SourceOffset) -> Result<TypeId, Error> {
        Ok(self.object(id, location)?.ty)
    }
    pub(super) fn valid_pointer(
        &self,
        value: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Root::Object(id) = value.root {
            self.object(id, location).map_err(|_| {
                nonconstant(location, "a pointer to an object whose lifetime has ended")
            })?;
        }
        Ok(())
    }
    pub(super) fn add_cell(
        &mut self,
        place: Address,
        value: Option<TemporaryValue>,
        location: SourceOffset,
    ) -> Result<CellId, Error> {
        self.steps += 1;
        if self.steps >= 1 << 24 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its storage budget",
            ));
        }
        let Root::Object(object) = place.root else {
            return Err(Error::Invariant("subobject outside evaluation storage"));
        };
        let id = CellId(
            u32::try_from(self.cells.len()).map_err(|_| Error::Limit("constant cell limit"))?,
        );
        self.index
            .entry(Key(object, place.offset, place.designated, place.bit_field))
            .or_insert(id);
        self.cells.push(Cell {
            ty: place.designated,
            offset: place.offset,
            field: place.bit_field,
            value,
            end: self.cells.len() + 1,
        });
        self.objects[object.slot as usize].cells.end = self.cells.len();
        Ok(id)
    }
    pub(super) fn finish_cell(&mut self, id: CellId) {
        self.cells[id.0 as usize].end = self.cells.len();
    }
    pub(super) fn locate(&self, place: Address, location: SourceOffset) -> Result<CellId, Error> {
        let Root::Object(object) = place.root else {
            return Err(Error::Invariant("frame access outside object storage"));
        };
        self.object(object, location)?;
        if place.past_end {
            return Err(nonconstant(
                location,
                "an access one past the end of an array",
            ));
        }
        self.index
            .get(&Key(
                object,
                place.offset,
                place.designated,
                place.bit_field,
            ))
            .copied()
            .ok_or_else(|| {
                nonconstant(
                    location,
                    "an access outside the designated object or to an inactive union member",
                )
            })
    }
    pub(super) fn cells(
        &self,
        place: Address,
        location: SourceOffset,
    ) -> Result<std::ops::Range<usize>, Error> {
        let id = self.locate(place, location)?.0 as usize;
        Ok(id..self.cells[id].end)
    }
    pub(super) fn cell_place(&self, root: Address, index: usize) -> Address {
        let cell = self.cells[index];
        Address {
            offset: cell.offset,
            designated: cell.ty,
            bit_field: cell.field,
            ..root
        }
    }
    pub(super) fn read(
        &self,
        place: Address,
        location: SourceOffset,
    ) -> Result<TemporaryValue, Error> {
        let id = self.locate(place, location)?;
        self.cells[id.0 as usize]
            .value
            .ok_or_else(|| nonconstant(location, "a read of an object that is not initialized"))
    }
    pub(super) fn write(
        &mut self,
        place: Address,
        value: TemporaryValue,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let id = self.locate(place, location)?;
        self.cells[id.0 as usize].value = Some(value);
        Ok(())
    }
    pub(super) fn copy(
        &mut self,
        to: Address,
        from: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let target = self.cells(to, location)?;
        let source = self.cells(from, location)?;
        if target.len() != source.len() {
            return Err(Error::Invariant("incompatible constant object copy"));
        }
        for (to, from) in target.zip(source) {
            self.cells[to].value = self.cells[from].value;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword,
        sema::{constants::Constant, graph::Graph, types::Scalar},
    };

    #[test]
    fn released_slots_cannot_alias_new_objects_and_recursive_bindings_are_local() {
        let mut graph = Graph::new();
        let ty = graph.types.scalar(Scalar::Bool).unwrap();
        let entity = graph.add_entity(Some(ty), None, Keyword::Bool).unwrap();
        let location = SourceOffset(0);
        let mut storage = Frames::default();
        let empty = storage.region();
        let first = storage
            .allocate(
                ty,
                Some(TemporaryValue::Integer(Constant { bits: 1, ty })),
                location,
            )
            .unwrap();
        let mut outer = Frame::new(ty, graph.global);
        outer.bind(entity, first);
        storage.push(outer);
        let region = storage.region();
        let second = storage.allocate(ty, None, location).unwrap();
        let mut inner = Frame::new(ty, graph.global);
        inner.bind(entity, second);
        storage.push(inner);
        assert_eq!(storage.local(entity).unwrap().root, second.root);
        assert!(matches!(
            storage.read(second, location),
            Err(Error::NonConstant {
                message: "a read of an object that is not initialized",
                ..
            })
        ));
        storage.pop();
        storage.release(region);
        assert_eq!(storage.local(entity).unwrap().root, first.root);
        let reused = storage.allocate(ty, None, location).unwrap();
        assert_ne!(second.root, reused.root);
        assert!(matches!(
            storage.read(second, location),
            Err(Error::NonConstant {
                message: "an access to an object whose lifetime has ended",
                ..
            })
        ));
        assert!(matches!(
            storage.read(first, location),
            Ok(TemporaryValue::Integer(Constant { bits: 1, .. }))
        ));
        storage.pop();
        storage.release(empty);
        assert!(storage.objects.is_empty());
        assert!(!storage.active());
    }
    #[test]
    fn subobjects_use_typed_offsets_and_bulk_copy_has_independent_cells() {
        let mut graph = Graph::new();
        let integer = graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))
            .unwrap();
        let class = graph.add_entity(None, None, Keyword::Struct).unwrap();
        let ty = graph
            .types
            .intern(
                super::super::types::TypeKind::Class(class),
                super::super::types::Cv::default(),
            )
            .unwrap();
        let location = SourceOffset(0);
        let mut storage = Frames::default();
        let region = storage.region();
        let mut objects = Vec::new();
        for _ in 0..2 {
            let root = storage.allocate(ty, None, location).unwrap();
            let cell = storage.locate(root, location).unwrap();
            for (offset, bits) in [(0, 3), (4, 7)] {
                let place = Address {
                    designated: integer,
                    offset,
                    ..root
                };
                storage
                    .add_cell(
                        place,
                        Some(TemporaryValue::Integer(Constant { bits, ty: integer })),
                        location,
                    )
                    .unwrap();
            }
            storage.finish_cell(cell);
            objects.push(root);
        }
        storage.copy(objects[1], objects[0], location).unwrap();
        let source = Address {
            designated: integer,
            offset: 4,
            ..objects[0]
        };
        storage
            .write(
                source,
                TemporaryValue::Integer(Constant {
                    bits: 9,
                    ty: integer,
                }),
                location,
            )
            .unwrap();
        let target = Address {
            designated: integer,
            offset: 4,
            ..objects[1]
        };
        assert!(matches!(
            storage.read(target, location),
            Ok(TemporaryValue::Integer(Constant { bits: 7, .. }))
        ));
        assert!(
            storage
                .read(
                    Address {
                        offset: 2,
                        ..target
                    },
                    location
                )
                .is_err()
        );
        storage.release(region);
        assert!(storage.index.is_empty());
        assert!(storage.cells.is_empty());
        assert!(storage.read(target, location).is_err());
    }
}
