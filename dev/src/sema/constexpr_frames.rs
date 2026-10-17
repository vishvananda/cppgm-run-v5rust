// Translated from sema/constant_frames.h and constant_interpreter.cpp. See NOTICE.
//! Evaluation-owned scalar objects and call frames. Reused slots carry generations.
use super::{
    Error,
    constant_addresses::{Address, Root, TemporaryValue, nonconstant},
    graph::{EntityId, ScopeId},
    types::TypeId,
};
use crate::support::source::SourceOffset;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ObjectId {
    slot: u32,
    generation: u64,
}

struct Object {
    generation: u64,
    ty: TypeId,
    value: Option<TemporaryValue>,
}
pub(super) struct Frame {
    pub result: TypeId,
    pub scope: ScopeId,
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
    locals: usize,
}
#[derive(Default)]
pub(super) struct Frames {
    objects: Vec<Object>,
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
            locals: self.stack.last().map_or(0, |f| f.locals.len()),
        }
    }
    pub(super) fn release(&mut self, region: Region) {
        if let Some(frame) = self.stack.last_mut() {
            for entity in frame.locals.drain(region.locals..) {
                frame.places.remove(&entity);
            }
        }
        self.objects.truncate(region.objects);
    }
    pub(super) fn clear(&mut self) {
        self.objects.clear();
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
        // A scalar skeleton contributes one value and one step. The C++
        // leaf budget is per object; arrays/classes have a separate producer.
        self.steps += 1;
        if self.steps >= 1 << 24 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its storage budget",
            ));
        }
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
            generation: id.generation,
            ty,
            value,
        });
        Ok(Address::new(Root::Object(id), ty))
    }
    fn object(&self, id: ObjectId, location: SourceOffset) -> Result<&Object, Error> {
        self.objects
            .get(id.slot as usize)
            .filter(|object| object.generation == id.generation)
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
    pub(super) fn read(
        &self,
        place: Address,
        location: SourceOffset,
    ) -> Result<TemporaryValue, Error> {
        let Root::Object(id) = place.root else {
            return Err(Error::Invariant("frame read outside object storage"));
        };
        let object = self.object(id, location)?;
        if place.offset != 0 || place.past_end {
            return Err(nonconstant(
                location,
                "an access outside the designated object or to an inactive union member",
            ));
        }
        object
            .value
            .ok_or_else(|| nonconstant(location, "a read of an object that is not initialized"))
    }
    pub(super) fn write(
        &mut self,
        place: Address,
        value: TemporaryValue,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let Root::Object(id) = place.root else {
            return Err(Error::Invariant("frame write outside object storage"));
        };
        self.object(id, location)?;
        if place.offset != 0 || place.past_end {
            return Err(nonconstant(
                location,
                "an access outside the designated object or to an inactive union member",
            ));
        }
        self.objects[id.slot as usize].value = Some(value);
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
}
