// Translated from sema/constant_frames.h and constant_interpreter.cpp. See NOTICE.
//! Evaluation-owned objects and call frames. Reused slots carry generations.
use super::{
    Error,
    constant_addresses::{Address, Root, Selection, TemporaryValue, Variant, nonconstant},
    constant_pointer_operations::MemberSpans,
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
#[derive(Clone, Copy, Debug)]
pub(super) struct VariantId(u32);
#[derive(Clone, Copy)]
struct Cell {
    ty: TypeId,
    offset: i64,
    field: Option<EntityId>,
    value: Option<TemporaryValue>,
    end: usize,
    parent: Option<CellId>,
    member: Option<EntityId>,
    union: bool,
    class_spans: Option<EntityId>,
    active: Option<CellId>,
    next: Option<CellId>,
}
/// A completed object's cells, detached from evaluation slot generations.
pub(super) struct Frozen {
    cells: Vec<Cell>,
    index: HashMap<(i64, TypeId, Option<EntityId>), CellId>,
    members: HashMap<(CellId, EntityId), CellId>,
}
pub(super) struct PlacementValue {
    pub depth: usize,
    pub ty: TypeId,
    pub offset: i64,
    pub field: Option<EntityId>,
    pub value: Option<TemporaryValue>,
}
impl Frozen {
    fn locate(
        &self,
        spans: &MemberSpans,
        place: Address,
        named: Option<Variant>,
        location: SourceOffset,
    ) -> Result<usize, Error> {
        resolve(
            spans,
            &self.cells,
            place,
            named,
            false,
            location,
            (
                |place: Address| {
                    self.index
                        .get(&(place.offset, place.designated, place.bit_field()))
                        .copied()
                },
                |owner, member| self.members.get(&(owner, member)).copied(),
            ),
        )
        .map(|id| id.0 as usize)
    }
    pub(super) fn read(
        &self,
        spans: &MemberSpans,
        place: Address,
        named: Option<Variant>,
        location: SourceOffset,
    ) -> Result<TemporaryValue, Error> {
        self.cells[self.locate(spans, place, named, location)?]
            .value
            .ok_or_else(|| nonconstant(location, "a read of an object that is not initialized"))
    }
    pub(super) fn complete(&self, aggregate: impl Fn(TypeId) -> bool) -> bool {
        self.cells.iter().enumerate().all(|(i, cell)| {
            let Ok(id) = u32::try_from(i) else {
                return false;
            };
            !accessible(&self.cells, CellId(id), 0)
                || if cell.union {
                    cell.active.is_some() || cell.end == i + 1
                } else {
                    aggregate(cell.ty) || cell.value.is_some()
                }
        })
    }
    #[cfg(test)]
    pub(super) fn values(
        &self,
    ) -> impl Iterator<Item = (TypeId, i64, Option<EntityId>, Option<TemporaryValue>)> + '_ {
        self.cells.iter().enumerate().filter_map(|(i, cell)| {
            let id = CellId(u32::try_from(i).ok()?);
            accessible(&self.cells, id, 0).then_some((cell.ty, cell.offset, cell.field, cell.value))
        })
    }
    // The data writer has its own structural depth bound. Semantic reads
    // keep the full value, including subobjects beyond that bound.
    pub(super) fn placement_values(&self) -> impl Iterator<Item = PlacementValue> + '_ {
        self.cells
            .iter()
            .enumerate()
            .scan(Vec::new(), |ends: &mut Vec<usize>, (index, cell)| {
                while ends.last().is_some_and(|&end| end <= index) {
                    ends.pop();
                }
                let depth = ends.len();
                ends.push(cell.end);
                Some((index, depth, cell))
            })
            .filter_map(|(index, depth, cell)| {
                let id = CellId(u32::try_from(index).ok()?);
                accessible(&self.cells, id, 0).then_some(PlacementValue {
                    depth,
                    ty: cell.ty,
                    offset: cell.offset,
                    field: cell.field,
                    value: cell.value,
                })
            })
    }
}
struct Object {
    id: ObjectId,
    alive: bool,
    initializing_declaration: bool,
    ty: TypeId,
    cells: std::ops::Range<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key(ObjectId, i64, TypeId, Option<EntityId>);
pub(super) struct Frame {
    pub source_function: Option<super::predefined_names::SourceFunction>,
    pub result: TypeId,
    pub scope: ScopeId,
    pub object: Option<Address>,
    pub destination: Option<Address>,
    pub parameters: Vec<Address>,
    pub role: FrameRole,
    pub peak_depth: usize,
    parent_call: Option<usize>,
    seeking: Option<crate::syntax::ast::StmtId>,
    places: HashMap<EntityId, Address>,
    locals: Vec<EntityId>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FrameRole {
    Ordinary,
    Constructor,
    Initializer,
}
impl Frame {
    pub(super) fn new(result: TypeId, scope: ScopeId, role: FrameRole) -> Self {
        Self {
            result,
            scope,
            source_function: None,
            seeking: None,
            object: None,
            destination: None,
            parameters: Vec::new(),
            role,
            peak_depth: 0,
            parent_call: None,
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
    frame: Option<usize>,
}
#[derive(Default)]
struct Control {
    floor: usize,
    depth: usize,
    call: Option<usize>,
    steps: u64,
}
pub(super) struct EvaluationSave {
    control: Control,
    region: Region,
    stack: usize,
}
#[derive(Default)]
pub(super) struct Frames {
    objects: Vec<Object>,
    cells: Vec<Cell>,
    index: HashMap<Key, CellId>,
    stack: Vec<Frame>,
    generation: u64,
    control: Control,
    unions: Box<Unions>,
}
// Sparse union metadata has one evaluation owner, keeping the ordinary
// interpreter/frame handles small as semantic queries move through the driver.
#[derive(Default)]
struct Unions {
    members: HashMap<(CellId, EntityId), CellId>,
    designations: Vec<Variant>,
    designation_index: HashMap<Variant, VariantId>,
    activation_path: Vec<(CellId, CellId)>,
}
impl Frames {
    pub(super) fn freeze(
        &self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
        mut publish: impl FnMut(TemporaryValue) -> Result<TemporaryValue, Error>,
    ) -> Result<Frozen, Error> {
        let range = self.cells(spans, place, location)?;
        let mut cells = self.cells[range.clone()].to_vec();
        let mut index = HashMap::with_capacity(cells.len());
        let first = u32::try_from(range.start)
            .map_err(|_| Error::Limit("published constant cell limit"))?;
        for (i, cell) in cells.iter_mut().enumerate() {
            cell.offset = cell.offset.wrapping_sub(place.offset);
            cell.end -= range.start;
            cell.parent = cell
                .parent
                .filter(|id| id.0 as usize >= range.start)
                .map(|id| CellId(id.0 - first));
            cell.active = cell.active.map(|id| CellId(id.0 - first));
            cell.next = index.insert(
                (cell.offset, cell.ty, cell.field),
                CellId(
                    u32::try_from(i).map_err(|_| Error::Limit("published constant cell limit"))?,
                ),
            );
        }
        // Inactive storage is retained for copies, but cannot publish an
        // escaping pointer or contribute an item to the completed image.
        for i in 0..cells.len() {
            if accessible(
                &cells,
                CellId(u32::try_from(i).map_err(|_| Error::Limit("constant cell limit"))?),
                0,
            ) {
                cells[i].value = cells[i].value.map(&mut publish).transpose()?;
            }
        }
        let mut members = HashMap::new();
        for (i, cell) in cells.iter().enumerate() {
            if let (Some(parent), Some(member)) = (cell.parent, cell.member)
                && cells[parent.0 as usize].union
            {
                members.insert(
                    (parent, member),
                    CellId(u32::try_from(i).map_err(|_| Error::Limit("constant cell limit"))?),
                );
            }
        }
        Ok(Frozen {
            cells,
            index,
            members,
        })
    }
    pub(super) fn copy_published(
        &mut self,
        spans: &MemberSpans,
        to: Address,
        from: Address,
        value: &Frozen,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let first = value.locate(spans, from, self.designation(from.variant()), location)?;
        let source = first..value.cells[first].end;
        let target = self.copy_destination(spans, to, location)?;
        if target.len() != source.len() {
            return Err(Error::Invariant(
                "incompatible published constant object copy",
            ));
        }
        let (source_start, target_start) = (source.start, target.start);
        for (to, from) in target.zip(source) {
            self.copy_cell(to, value.cells[from], source_start, target_start)?;
        }
        Ok(())
    }
    pub(super) fn seeking(&self) -> Option<crate::syntax::ast::StmtId> {
        self.current().ok()?.seeking
    }
    pub(super) fn seek(&mut self, id: Option<crate::syntax::ast::StmtId>) {
        if self.active()
            && let Some(frame) = self.stack.last_mut()
        {
            frame.seeking = id;
        }
    }
    pub(super) fn active(&self) -> bool {
        self.stack.len() > self.control.floor
    }
    pub(super) fn depth(&self) -> usize {
        self.control.depth
    }
    pub(super) fn current(&self) -> Result<&Frame, Error> {
        self.stack
            .get(self.control.floor..)
            .and_then(|frames| frames.last())
            .ok_or(Error::Invariant("constant body without frame"))
    }
    pub(super) fn current_call(&self) -> Option<&Frame> {
        self.control.call.and_then(|index| self.stack.get(index))
    }
    pub(super) fn bind(&mut self, entity: EntityId, place: Address) -> Result<(), Error> {
        self.stack
            .get_mut(self.control.floor..)
            .and_then(|frames| frames.last_mut())
            .ok_or(Error::Invariant("constant local without frame"))?
            .bind(entity, place);
        Ok(())
    }
    pub(super) fn local(&self, entity: EntityId) -> Option<Address> {
        self.current().ok()?.places.get(&entity).copied()
    }
    pub(super) fn push(&mut self, mut frame: Frame) {
        if frame.role != FrameRole::Initializer {
            frame.parent_call = self.control.call;
            self.control.call = Some(self.stack.len());
            self.control.depth += 1;
            if frame.role == FrameRole::Ordinary {
                frame.peak_depth = self.control.depth;
            }
        }
        self.stack.push(frame);
    }
    pub(super) fn pop(&mut self) {
        if !self.active() {
            return;
        }
        if let Some(frame) = self.stack.pop()
            && frame.role != FrameRole::Initializer
        {
            self.control.depth -= 1;
            self.control.call = frame.parent_call;
            if frame.role == FrameRole::Ordinary
                && let Some(parent) = frame
                    .parent_call
                    .and_then(|index| self.stack.get_mut(index))
            {
                parent.peak_depth = parent.peak_depth.max(frame.peak_depth);
            }
        }
    }
    pub(super) fn region(&self) -> Region {
        Region {
            objects: self.objects.len(),
            cells: self.cells.len(),
            locals: self.current().map_or(0, |f| f.locals.len()),
            frame: self.active().then(|| self.stack.len() - 1),
        }
    }
    pub(super) fn release(&mut self, region: Region) {
        if let Some(frame) = region.frame.and_then(|index| self.stack.get_mut(index)) {
            for entity in frame.locals.drain(region.locals..) {
                frame.places.remove(&entity);
            }
        }
        for index in region.cells..self.cells.len() {
            if let (Some(parent), Some(member)) =
                (self.cells[index].parent, self.cells[index].member)
            {
                self.unions.members.remove(&(parent, member));
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
        self.unions.members.clear();
        self.unions.designations.clear();
        self.unions.designation_index.clear();
        self.unions.activation_path.clear();
        self.stack.clear();
        self.control = Control::default();
    }
    pub(super) fn enter_evaluation(&mut self) -> EvaluationSave {
        let control = std::mem::replace(
            &mut self.control,
            Control {
                floor: self.stack.len(),
                ..Control::default()
            },
        );
        EvaluationSave {
            control,
            region: self.region(),
            stack: self.stack.len(),
        }
    }
    pub(super) fn leave_evaluation(&mut self, save: EvaluationSave) -> u64 {
        let steps = self.control.steps;
        self.stack.truncate(save.stack);
        self.release(save.region);
        self.control = save.control;
        steps
    }
    pub(super) fn step(&mut self, location: SourceOffset) -> Result<(), Error> {
        self.control.steps += 1;
        if self.control.steps > 1 << 24 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its step budget",
            ));
        }
        Ok(())
    }
    pub(super) fn skeleton_step(&mut self) {
        self.control.steps += 1;
    }
    pub(super) fn array_skeleton_fits(&mut self, bound: u64) -> bool {
        if bound > 1 << 22 || self.control.steps.saturating_add(bound) > 1 << 24 {
            self.control.steps = 1 << 24;
            return false;
        }
        true
    }
    pub(super) fn check_storage_budget(&self, location: SourceOffset) -> Result<(), Error> {
        if self.control.steps >= 1 << 24 {
            return Err(nonconstant(
                location,
                "constant evaluation exceeds its storage budget",
            ));
        }
        Ok(())
    }
    pub(super) fn allocate(
        &mut self,
        (ty, cell_type): (TypeId, TypeId),
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
            alive: true,
            initializing_declaration: false,
            ty,
            cells: self.cells.len()..self.cells.len(),
        });
        let place = Address::new(Root::Object(id), cell_type);
        self.add_cell(place, value, location)?;
        Ok(place)
    }
    pub(super) fn initialize_declaration(&mut self, place: Address) -> Result<(), Error> {
        let Root::Object(id) = place.root else {
            return Err(Error::Invariant(
                "declaration initialization outside storage",
            ));
        };
        self.objects[id.slot as usize].initializing_declaration = true;
        Ok(())
    }
    pub(super) fn initializing_declaration(
        &self,
        place: Address,
        location: SourceOffset,
    ) -> Result<bool, Error> {
        let Root::Object(id) = place.root else {
            return Ok(false);
        };
        Ok(place.offset == 0 && self.object(id, location)?.initializing_declaration)
    }
    fn object(&self, id: ObjectId, location: SourceOffset) -> Result<&Object, Error> {
        self.objects
            .get(id.slot as usize)
            .filter(|object| object.id == id && object.alive)
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
    pub(super) fn end_lifetime(&mut self, place: Address) {
        if let Root::Object(id) = place.root
            && let Some(object) = self.objects.get_mut(id.slot as usize)
            && object.id == id
        {
            object.alive = false;
        }
    }
    pub(super) fn add_cell(
        &mut self,
        place: Address,
        value: Option<TemporaryValue>,
        _location: SourceOffset,
    ) -> Result<CellId, Error> {
        let Root::Object(object) = place.root else {
            return Err(Error::Invariant("subobject outside evaluation storage"));
        };
        let id = CellId(
            u32::try_from(self.cells.len()).map_err(|_| Error::Limit("constant cell limit"))?,
        );
        let next = self.index.insert(
            Key(object, place.offset, place.designated, place.bit_field()),
            id,
        );
        self.cells.push(Cell {
            ty: place.designated,
            offset: place.offset,
            field: place.bit_field(),
            value: value.filter(|value| !value.indeterminate()),
            end: self.cells.len() + 1,
            parent: None,
            member: None,
            union: false,
            class_spans: None,
            active: None,
            next,
        });
        self.objects[object.slot as usize].cells.end = self.cells.len();
        Ok(id)
    }
    pub(super) fn finish_cell(&mut self, id: CellId) {
        self.cells[id.0 as usize].end = self.cells.len();
    }
    pub(super) fn add_child(
        &mut self,
        parent: CellId,
        place: Address,
        member: Option<EntityId>,
        location: SourceOffset,
    ) -> Result<CellId, Error> {
        let child = self.add_cell(place, None, location)?;
        self.cells[child.0 as usize].parent = Some(parent);
        self.cells[child.0 as usize].member = member;
        if self.cells[parent.0 as usize].union
            && let Some(member) = member
        {
            self.unions.members.insert((parent, member), child);
        }
        Ok(child)
    }
    pub(super) fn mark_class(&mut self, cell: CellId, class: EntityId) {
        self.cells[cell.0 as usize].class_spans = Some(class);
    }
    pub(super) fn mark_union(&mut self, cell: CellId) {
        self.cells[cell.0 as usize].union = true;
    }
    pub(super) fn locate(
        &self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<CellId, Error> {
        self.resolve(spans, place, false, location)
    }
    fn resolve(
        &self,
        spans: &MemberSpans,
        place: Address,
        write: bool,
        location: SourceOffset,
    ) -> Result<CellId, Error> {
        let Root::Object(object) = place.root else {
            return Err(Error::Invariant("frame access outside object storage"));
        };
        self.object(object, location)?;
        resolve(
            spans,
            &self.cells,
            place,
            self.designation(place.variant()),
            write,
            location,
            (
                |place: Address| {
                    self.index
                        .get(&Key(
                            object,
                            place.offset,
                            place.designated,
                            place.bit_field(),
                        ))
                        .copied()
                },
                |owner, member| self.unions.members.get(&(owner, member)).copied(),
            ),
        )
    }
    pub(super) fn designate(
        &mut self,
        member: EntityId,
        owner: TypeId,
        offset: i64,
    ) -> Result<VariantId, Error> {
        let key = Variant {
            member,
            owner,
            offset,
        };
        if let Some(&id) = self.unions.designation_index.get(&key) {
            return Ok(id);
        }
        let id = VariantId(
            u32::try_from(self.unions.designations.len())
                .map_err(|_| Error::Limit("constant designation limit"))?,
        );
        self.unions.designations.push(key);
        self.unions.designation_index.insert(key, id);
        Ok(id)
    }
    pub(super) fn designation(&self, id: Option<VariantId>) -> Option<Variant> {
        id.map(|id| self.unions.designations[id.0 as usize])
    }
    pub(super) fn activate(
        &mut self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<CellId, Error> {
        let id = self.resolve(spans, place, true, location)?;
        let mut path = std::mem::take(&mut self.unions.activation_path);
        let mut child = id;
        while let Some(parent) = self.cells[child.0 as usize].parent {
            if self.cells[parent.0 as usize].union && self.cells[child.0 as usize].field.is_none() {
                path.push((parent, child));
            }
            child = parent;
        }
        for (parent, child) in path.drain(..).rev() {
            if self.cells[parent.0 as usize].active != Some(child) {
                let start = child.0 as usize;
                let end = self.cells[start].end;
                for cell in &mut self.cells[start..end] {
                    cell.value = None;
                    cell.active = None;
                }
                self.cells[parent.0 as usize].active = Some(child);
            }
        }
        self.unions.activation_path = path;
        Ok(id)
    }
    pub(super) fn zero_cells(
        &mut self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<Vec<CellId>, Error> {
        let root = self.activate(spans, place, location)?;
        let mut work = vec![root.0 as usize];
        let mut leaves = Vec::new();
        while let Some(index) = work.pop() {
            let mut child = index + 1;
            if self.cells[index].union {
                if child < self.cells[index].end {
                    self.cells[index].active = Some(CellId(
                        u32::try_from(child).map_err(|_| Error::Limit("constant cell limit"))?,
                    ));
                    work.push(child);
                }
                continue;
            }
            if child == self.cells[index].end {
                leaves.push(CellId(
                    u32::try_from(index).map_err(|_| Error::Limit("constant cell limit"))?,
                ));
            }
            while child < self.cells[index].end {
                work.push(child);
                child = self.cells[child].end;
            }
        }
        Ok(leaves)
    }
    pub(super) fn active_cells(
        &self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<Vec<CellId>, Error> {
        let range = self.cells(spans, place, location)?;
        range
            .filter_map(|i| match u32::try_from(i) {
                Ok(id) => accessible(&self.cells, CellId(id), 0).then_some(Ok(CellId(id))),
                Err(_) => Some(Err(Error::Limit("constant cell limit"))),
            })
            .collect()
    }
    fn cells(
        &self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<std::ops::Range<usize>, Error> {
        let id = self.locate(spans, place, location)?.0 as usize;
        Ok(id..self.cells[id].end)
    }
    pub(super) fn cell_place(&self, root: Address, index: CellId) -> Address {
        let cell = self.cells[index.0 as usize];
        Address {
            offset: cell.offset,
            designated: cell.ty,
            selection: cell.field.map_or(Selection::None, Selection::BitField),
            ..root
        }
    }
    // zero_into traverses the already selected object, as copying does. It
    // does not resolve each leaf address again through overlapping members.
    pub(super) fn zero_value(&mut self, cell: CellId, value: TemporaryValue) {
        self.cells[cell.0 as usize].value = Some(value);
    }
    pub(super) fn read(
        &self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<TemporaryValue, Error> {
        self.read_value(spans, place, location)?
            .ok_or_else(|| nonconstant(location, "a read of an object that is not initialized"))
    }
    pub(super) fn read_value(
        &self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<Option<TemporaryValue>, Error> {
        let id = self.locate(spans, place, location)?;
        Ok(self.cells[id.0 as usize].value)
    }
    pub(super) fn write(
        &mut self,
        spans: &MemberSpans,
        place: Address,
        value: TemporaryValue,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let id = self.activate(spans, place, location)?;
        self.cells[id.0 as usize].value = (!value.indeterminate()).then_some(value);
        Ok(())
    }
    pub(super) fn copy(
        &mut self,
        spans: &MemberSpans,
        to: Address,
        from: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let source = self.cells(spans, from, location)?;
        let target = self.copy_destination(spans, to, location)?;
        if target.len() != source.len() {
            return Err(Error::Invariant("incompatible constant object copy"));
        }
        let (source_start, target_start) = (source.start, target.start);
        for (to, from) in target.zip(source) {
            self.copy_cell(to, self.cells[from], source_start, target_start)?;
        }
        Ok(())
    }
    fn copy_cell(
        &mut self,
        to: usize,
        from: Cell,
        source_start: usize,
        target_start: usize,
    ) -> Result<(), Error> {
        self.cells[to].value = from.value;
        self.cells[to].active = from
            .active
            .map(|id| {
                let index = (id.0 as usize)
                    .checked_sub(source_start)
                    .and_then(|index| index.checked_add(target_start))
                    .and_then(|index| u32::try_from(index).ok())
                    .ok_or(Error::Invariant(
                        "constant copy active member outside storage",
                    ))?;
                Ok(CellId(index))
            })
            .transpose()?;
        Ok(())
    }
    fn copy_destination(
        &mut self,
        spans: &MemberSpans,
        place: Address,
        location: SourceOffset,
    ) -> Result<std::ops::Range<usize>, Error> {
        if matches!(place.root, Root::Variable(_)) {
            return Err(nonconstant(
                location,
                "a modification of an object whose lifetime began outside the evaluation",
            ));
        }
        let id = self.activate(spans, place, location)?;
        Ok(id.0 as usize..self.cells[id.0 as usize].end)
    }
}

// Typed indexes find the possible subobjects; the parent links resolve overlap
// by the active union path. Ordinary nonoverlapping cells retain one lookup.
fn accessible(cells: &[Cell], mut child: CellId, first: usize) -> bool {
    while let Some(parent) = cells[child.0 as usize].parent {
        if (parent.0 as usize) < first {
            break;
        }
        if cells[parent.0 as usize].union && cells[parent.0 as usize].active != Some(child) {
            return false;
        }
        child = parent;
    }
    true
}
fn preference(
    cells: &[Cell],
    mut child: CellId,
    first: usize,
) -> impl Iterator<Item = (bool, u32)> + '_ {
    std::iter::from_fn(move || {
        while let Some(parent) = cells[child.0 as usize].parent {
            if (parent.0 as usize) < first {
                return None;
            }
            let prior = child;
            child = parent;
            if cells[parent.0 as usize].union {
                return Some((cells[parent.0 as usize].active != Some(prior), prior.0));
            }
        }
        None
    })
}
fn compare_preference(cells: &[Cell], a: CellId, b: CellId, first: usize) -> std::cmp::Ordering {
    let a_len = preference(cells, a, first).count();
    let b_len = preference(cells, b, first).count();
    // Parent links yield the innermost union first. Align the outer paths,
    // then let each outer difference supersede an inner difference. Equal
    // prefixes prefer the shorter path, just as an outer-first lexical walk.
    // No candidate allocates a temporary path, even with nested overlaps.
    preference(cells, a, first)
        .skip(a_len.saturating_sub(b_len))
        .zip(preference(cells, b, first).skip(b_len.saturating_sub(a_len)))
        .fold(a_len.cmp(&b_len), |order, (a, b)| match a.cmp(&b) {
            std::cmp::Ordering::Equal => order,
            other => other,
        })
}
fn find(
    spans: &MemberSpans,
    cells: &[Cell],
    mut head: Option<CellId>,
    range: std::ops::Range<usize>,
    write: bool,
    field: bool,
) -> Result<Option<CellId>, Error> {
    let mut best = None;
    let owner = |id: CellId| {
        if field {
            cells[id.0 as usize].parent.unwrap_or(id)
        } else {
            id
        }
    };
    while let Some(id) = head {
        let index = id.0 as usize;
        head = cells[index].next;
        if range.contains(&index)
            && selected_member_path(spans, cells, owner(id), range.start)?
            && (write || accessible(cells, owner(id), range.start))
            && best.is_none_or(|prior: CellId| {
                compare_preference(cells, owner(id), owner(prior), range.start)
                    .then_with(|| id.0.cmp(&prior.0))
                    .is_lt()
            })
        {
            best = Some(id);
        }
    }
    Ok(best)
}

// An index hit is only a possible subobject. Non-union class descent selects
// the member owning the byte first, before checking its contained type.
fn selected_member_path(
    spans: &MemberSpans,
    cells: &[Cell],
    mut child: CellId,
    first: usize,
) -> Result<bool, Error> {
    let offset = cells[child.0 as usize].offset;
    while let Some(parent) = cells[child.0 as usize].parent {
        if (parent.0 as usize) < first {
            break;
        }
        let node = cells[child.0 as usize];
        let owner = cells[parent.0 as usize];
        if let (Some(class), Some(member), None) = (owner.class_spans, node.member, node.field)
            && !spans.permits(class, member, offset.wrapping_sub(owner.offset))?
        {
            return Ok(false);
        }
        child = parent;
    }
    Ok(true)
}

fn resolve(
    spans: &MemberSpans,
    cells: &[Cell],
    place: Address,
    named: Option<Variant>,
    write: bool,
    location: SourceOffset,
    (lookup, member_slot): (
        impl Fn(Address) -> Option<CellId>,
        impl Fn(CellId, EntityId) -> Option<CellId>,
    ),
) -> Result<CellId, Error> {
    if place.past_end {
        return Err(nonconstant(
            location,
            "an access one past the end of an array",
        ));
    }
    let outside = || {
        nonconstant(
            location,
            "an access outside the designated object or to an inactive union member",
        )
    };
    let mut range = 0..cells.len();
    let mut variant = false;
    if place.bit_field().is_none()
        && let Some(named) = named
    {
        let owner = Address {
            offset: named.offset,
            designated: named.owner,
            selection: Selection::None,
            ..place
        };
        let owner =
            find(spans, cells, lookup(owner), range.clone(), write, false)?.ok_or_else(outside)?;
        let first = owner.0 as usize;
        let child_id = member_slot(owner, named.member)
            .ok_or(Error::Invariant("union designation without member storage"))?;
        let child = child_id.0 as usize;
        if !write && cells[first].active != Some(child_id) {
            return Err(nonconstant(location, "a read of an inactive union member"));
        }
        range = child..cells[child].end;
        variant = true;
    }
    // Bit-fields have their own storage identity and deliberately do not
    // participate in the frozen union active-member record.
    if place.bit_field().is_some() {
        // Locate the containing class on its active path first. Bit-field
        // identity selects its cell without activating a direct union field.
        return find(spans, cells, lookup(place), range, write, true)?
            .ok_or_else(|| nonconstant(location, "an access outside the designated object"));
    }
    find(spans, cells, lookup(place), range, write, false)?.ok_or_else(|| {
        if variant {
            nonconstant(location, "an access outside the designated object")
        } else {
            outside()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword,
        sema::{constants::Constant, graph::Graph, types::Scalar},
    };

    #[test]
    fn independent_queries_hide_names_preserve_objects_and_restore_control() {
        let mut graph = Graph::new();
        let ty = graph.types.scalar(Scalar::Bool).unwrap();
        let entity = graph.add_entity(Some(ty), None, Keyword::Bool).unwrap();
        let location = SourceOffset(3);
        let mut frames = Frames::default();
        let outer = frames.allocate((ty, ty), None, location).unwrap();
        let mut frame = Frame::new(ty, graph.global, FrameRole::Ordinary);
        frame.bind(entity, outer);
        frames.push(frame);
        frames.step(location).unwrap();
        frames.step(location).unwrap();
        for fail in [false, true] {
            let save = frames.enter_evaluation();
            assert!(!frames.active());
            assert!(frames.current_call().is_none());
            assert!(frames.local(entity).is_none());
            assert_eq!(frames.depth(), 0);
            assert_eq!(frames.control.steps, 0);
            frames.valid_pointer(outer, location).unwrap();
            // A frameless query's region must not drain a suspended local.
            let region = frames.region();
            let child = frames.allocate((ty, ty), None, location).unwrap();
            frames.step(location).unwrap();
            frames.release(region);
            assert!(frames.valid_pointer(child, location).is_err());
            if fail {
                frames.control.steps = 1 << 24;
                assert!(frames.step(location).is_err());
            }
            let steps = frames.leave_evaluation(save);
            assert_eq!(steps, if fail { (1 << 24) + 1 } else { 1 });
            assert_eq!(frames.control.steps, 2);
            assert_eq!(frames.depth(), 1);
            assert_eq!(frames.local(entity).unwrap().root, outer.root);
            let reused = frames.allocate((ty, ty), None, location).unwrap();
            assert_ne!(child.root, reused.root);
            frames.valid_pointer(outer, location).unwrap();
        }
    }

    #[test]
    fn only_actual_calls_contribute_depth_and_constructor_peaks_do_not_propagate() {
        let mut graph = Graph::new();
        let ty = graph.types.scalar(Scalar::Bool).unwrap();
        let mut frames = Frames::default();
        frames.push(Frame::new(ty, graph.global, FrameRole::Ordinary));
        frames.push(Frame::new(ty, graph.global, FrameRole::Initializer));
        assert_eq!(frames.depth(), 1);
        assert!(frames.current_call().unwrap().role == FrameRole::Ordinary);
        frames.push(Frame::new(ty, graph.global, FrameRole::Constructor));
        assert_eq!(frames.depth(), 2);
        assert_eq!(frames.current().unwrap().peak_depth, 0);
        frames.push(Frame::new(ty, graph.global, FrameRole::Ordinary));
        frames.pop();
        assert_eq!(frames.current().unwrap().peak_depth, 3);
        frames.pop();
        frames.pop();
        assert_eq!(frames.current().unwrap().peak_depth, 1);
        frames.push(Frame::new(ty, graph.global, FrameRole::Ordinary));
        frames.push(Frame::new(ty, graph.global, FrameRole::Ordinary));
        frames.pop();
        frames.pop();
        assert_eq!(frames.current().unwrap().peak_depth, 3);
    }

    #[test]
    fn scalar_initialization_releases_success_and_failure_regions_inside_an_evaluation() {
        use crate::{
            preprocess::unit::Preprocessor,
            sema::analyzer::Analyzer,
            support::{source::SourceBuffer, stack::StackBudget},
            syntax::{SyntaxUnit, ast},
        };
        let source = SourceBuffer::new(b"constexpr int*p=0;constexpr double d=1.5;struct C{int member;};constexpr int C::*m=&C::member;int*unknown();int*bad=unknown();".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"scalar-regions").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let mut initializers = Vec::new();
        for &declaration in analyzer.tree.decl_lists.get(declarations) {
            if let ast::DeclKind::Simple { declarators, .. } =
                analyzer.tree.declarations.get(declaration).kind
            {
                for declarator in analyzer.tree.init_declarators.get(declarators) {
                    if let Some(initializer) = declarator.initializer {
                        let expression = analyzer.single_initializer(initializer).unwrap();
                        let source = analyzer.graph.expression_facts.get(expression).unwrap().ty;
                        initializers.push((source, initializer));
                    }
                }
            }
        }
        // The first initializer's integer zero initializes a pointer declaration.
        let pointer = analyzer.graph.types.pointer(initializers[0].0).unwrap();
        initializers[0].0 = pointer;
        let integer = initializers[0].1;
        let integer_type = analyzer
            .graph
            .expression_facts
            .get(analyzer.single_initializer(integer).unwrap())
            .unwrap()
            .ty;
        let expressions = analyzer.tree.expressions.mark();
        analyzer
            .in_constant_evaluation(|analyzer| {
                let outer = analyzer.allocate_constant_storage(
                    integer_type,
                    Some(TemporaryValue::Integer(Constant {
                        state: super::super::constant_addresses::ScalarState::Value,
                        ty: integer_type,
                        bits: 123,
                        high: 0,
                    })),
                    integer.location,
                )?;
                let objects = analyzer.constexpr_frames.objects.len();
                let cells = analyzer.constexpr_frames.cells.len();
                for (index, &(ty, initializer)) in initializers.iter().enumerate() {
                    let result = analyzer.initialized_scalar_value(ty, initializer, scope);
                    if index + 1 == initializers.len() {
                        assert!(matches!(result, Err(Error::NonConstant { .. })));
                    } else {
                        match result.unwrap() {
                            TemporaryValue::Pointer(value) => assert_eq!(value.root, Root::Null),
                            TemporaryValue::Floating(value) => assert_eq!(
                                value.value.bits(crate::types::float_value::Format::Double),
                                Some(0x3ff8_0000_0000_0000)
                            ),
                            TemporaryValue::MemberPointer(value) => assert!(value.member.is_some()),
                            TemporaryValue::Integer(_) => panic!("scalar declaration type"),
                        }
                    }
                    assert_eq!(analyzer.constexpr_frames.objects.len(), objects);
                    assert_eq!(analyzer.constexpr_frames.cells.len(), cells);
                    assert!(matches!(
                        analyzer.constexpr_frames.read(
                            &analyzer.graph.constant_member_spans,
                            outer,
                            integer.location
                        )?,
                        TemporaryValue::Integer(Constant { bits: 123, .. })
                    ));
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(analyzer.constexpr_frames.objects.is_empty());
        assert!(analyzer.constexpr_frames.index.is_empty());
    }

    #[test]
    fn union_cells_retain_variant_identity_across_publication_copy_and_release() {
        let mut graph = Graph::new();
        let ty = graph
            .types
            .scalar(Scalar::Basic(crate::types::FundamentalType::Int))
            .unwrap();
        let class = graph.add_entity(None, None, Keyword::Union).unwrap();
        let union_ty = graph
            .types
            .intern(
                super::super::types::TypeKind::Class(class),
                super::super::types::Cv::default(),
            )
            .unwrap();
        let a = graph.add_entity(Some(ty), None, Keyword::Int).unwrap();
        let b = graph.add_entity(Some(ty), None, Keyword::Int).unwrap();
        let location = SourceOffset(0);
        let mut frames = Frames::default();
        let empty = frames.region();
        // Nonzero source indices prove published parent/active/duplicate links
        // are rebased, rather than relying on the first object in a query.
        frames.allocate((ty, ty), None, location).unwrap();
        let source = frames
            .allocate((union_ty, union_ty), None, location)
            .unwrap();
        let root = frames
            .locate(&graph.constant_member_spans, source, location)
            .unwrap();
        frames.mark_union(root);
        for member in [a, b] {
            let id = frames.designate(member, union_ty, 0).unwrap();
            frames
                .add_child(
                    root,
                    Address {
                        designated: ty,
                        selection: Selection::Variant(id),
                        ..source
                    },
                    Some(member),
                    location,
                )
                .unwrap();
        }
        frames.finish_cell(root);
        let a_id = frames.designate(a, union_ty, 0).unwrap();
        let b_id = frames.designate(b, union_ty, 0).unwrap();
        let a_place = Address {
            designated: ty,
            selection: Selection::Variant(a_id),
            ..source
        };
        let b_place = Address {
            selection: Selection::Variant(b_id),
            ..a_place
        };
        frames
            .write(
                &graph.constant_member_spans,
                a_place,
                TemporaryValue::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
                    bits: 3,
                    ty,
                }),
                location,
            )
            .unwrap();
        assert!(matches!(
            frames.read(&graph.constant_member_spans, b_place, location),
            Err(Error::NonConstant {
                message: "a read of an inactive union member",
                ..
            })
        ));
        frames
            .write(
                &graph.constant_member_spans,
                b_place,
                TemporaryValue::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
                    bits: 7,
                    ty,
                }),
                location,
            )
            .unwrap();
        assert!(
            frames
                .read(&graph.constant_member_spans, a_place, location)
                .is_err()
        );
        let frozen = frames
            .freeze(&graph.constant_member_spans, source, location, Ok)
            .unwrap();
        assert!(frozen.complete(|t| t == union_ty));
        let target = frames
            .allocate((union_ty, union_ty), None, location)
            .unwrap();
        let root = frames
            .locate(&graph.constant_member_spans, target, location)
            .unwrap();
        frames.mark_union(root);
        for member in [a, b] {
            frames
                .add_child(
                    root,
                    Address {
                        designated: ty,
                        ..target
                    },
                    Some(member),
                    location,
                )
                .unwrap();
        }
        frames.finish_cell(root);
        frames
            .copy_published(
                &graph.constant_member_spans,
                target,
                source,
                &frozen,
                location,
            )
            .unwrap();
        let copied = Address {
            root: target.root,
            ..b_place
        };
        assert!(matches!(
            frames
                .read(&graph.constant_member_spans, copied, location)
                .unwrap(),
            TemporaryValue::Integer(Constant {
                high: 0,
                bits: 7,
                ..
            })
        ));
        frames
            .write(
                &graph.constant_member_spans,
                b_place,
                TemporaryValue::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
                    bits: 9,
                    ty,
                }),
                location,
            )
            .unwrap();
        assert!(matches!(
            frames
                .read(&graph.constant_member_spans, copied, location)
                .unwrap(),
            TemporaryValue::Integer(Constant {
                high: 0,
                bits: 7,
                ..
            })
        ));
        frames.release(empty);
        assert!(frames.unions.members.is_empty());
        assert!(frames.index.is_empty());
        frames.clear();
        assert!(frames.unions.designations.is_empty());
        assert!(frames.unions.designation_index.is_empty());
        assert!(frames.unions.activation_path.is_empty());
    }

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
                (ty, ty),
                Some(TemporaryValue::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
                    bits: 1,
                    ty,
                })),
                location,
            )
            .unwrap();
        let mut outer = Frame::new(ty, graph.global, FrameRole::Ordinary);
        outer.bind(entity, first);
        storage.push(outer);
        let region = storage.region();
        let second = storage.allocate((ty, ty), None, location).unwrap();
        let mut inner = Frame::new(ty, graph.global, FrameRole::Ordinary);
        inner.bind(entity, second);
        storage.push(inner);
        assert_eq!(storage.local(entity).unwrap().root, second.root);
        assert!(matches!(
            storage.read(&graph.constant_member_spans, second, location),
            Err(Error::NonConstant {
                message: "a read of an object that is not initialized",
                ..
            })
        ));
        storage.pop();
        storage.release(region);
        assert_eq!(storage.local(entity).unwrap().root, first.root);
        let reused = storage.allocate((ty, ty), None, location).unwrap();
        assert_ne!(second.root, reused.root);
        assert!(matches!(
            storage.read(&graph.constant_member_spans, second, location),
            Err(Error::NonConstant {
                message: "an access to an object whose lifetime has ended",
                ..
            })
        ));
        assert!(matches!(
            storage.read(&graph.constant_member_spans, first, location),
            Ok(TemporaryValue::Integer(Constant {
                high: 0,
                bits: 1,
                ..
            }))
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
            let root = storage.allocate((ty, ty), None, location).unwrap();
            let cell = storage
                .locate(&graph.constant_member_spans, root, location)
                .unwrap();
            for (offset, bits) in [(0, 3), (4, 7)] {
                let place = Address {
                    designated: integer,
                    offset,
                    ..root
                };
                storage
                    .add_cell(
                        place,
                        Some(TemporaryValue::Integer(Constant {
                            state: crate::sema::constant_addresses::ScalarState::Value,
                            high: 0,
                            bits,
                            ty: integer,
                        })),
                        location,
                    )
                    .unwrap();
            }
            storage.finish_cell(cell);
            objects.push(root);
        }
        storage
            .copy(
                &graph.constant_member_spans,
                objects[1],
                objects[0],
                location,
            )
            .unwrap();
        let source = Address {
            designated: integer,
            offset: 4,
            ..objects[0]
        };
        storage
            .write(
                &graph.constant_member_spans,
                source,
                TemporaryValue::Integer(Constant {
                    state: crate::sema::constant_addresses::ScalarState::Value,
                    high: 0,
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
            storage.read(&graph.constant_member_spans, target, location),
            Ok(TemporaryValue::Integer(Constant {
                high: 0,
                bits: 7,
                ..
            }))
        ));
        assert!(
            storage
                .read(
                    &graph.constant_member_spans,
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
        assert!(
            storage
                .read(&graph.constant_member_spans, target, location)
                .is_err()
        );
    }
}
