//! Sparse immutable expression payloads, owned by the semantic unit. See NOTICE.
use super::types::TypeId;
use super::{
    Error,
    expression_facts::{
        BaseCast, ClassInitialization, DestructorTarget, MemberAddressId, ObjectArgument,
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DetailsId(std::num::NonZeroU32);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct Details {
    pub(super) field_identity: Option<super::bit_fields::BitFieldId>,
    pub(super) query_value: Option<u64>,
    pub(super) reference_cast: Option<TypeId>,
    pub(super) intrinsic: Option<super::floating_intrinsics::FloatingIntrinsic>,
    pub(super) functions: Option<super::function_resolution::OverloadId>,
    pub(super) function: Option<super::graph::BindingId>,
    pub(super) call: Option<super::function_resolution::CallTarget>,
    pub(super) literal_call: Option<super::literal_calls::LiteralCall>,
    pub(super) destructor: Option<DestructorTarget>,
    pub(super) anonymous_member: Option<super::graph::BindingId>,
    pub(super) member: Option<super::graph::BindingId>,
    pub(super) member_naming: Option<super::graph::EntityId>,
    pub(super) object: Option<ObjectArgument>,
    pub(super) base_path: Option<super::inheritance::BasePathId>,
    pub(super) base_cast: Option<BaseCast>,
    pub(super) member_path: Option<super::inheritance::BasePathId>,
    pub(super) member_naming_path: Option<super::inheritance::BasePathId>,
    pub(super) member_address: Option<MemberAddressId>,
    pub(super) constructor: Option<super::construction::ConstructorId>,
    pub(super) class_initialization: Option<ClassInitialization>,
    pub(super) class_copy: Option<super::copy_initialization::CopySelection>,
}
struct Entry {
    data: Details,
    next: Option<DetailsId>,
}
#[derive(Default)]
pub(super) struct DetailsStore {
    entries: Vec<Entry>,
    heads: crate::support::hash::HashMap<u64, DetailsId>,
    empty: Details,
}
impl DetailsStore {
    pub(super) fn get(&self, id: Option<DetailsId>) -> &Details {
        id.map_or(&self.empty, |id| {
            &self.entries[(id.0.get() - 1) as usize].data
        })
    }
    pub(super) fn intern(&mut self, data: Details) -> Result<Option<DetailsId>, Error> {
        use std::hash::{Hash, Hasher};
        if data == self.empty {
            return Ok(None);
        }
        let mut hash = crate::support::hash::Hasher::default();
        data.hash(&mut hash);
        let key = hash.finish();
        let head = self.heads.get(&key).copied();
        let mut current = head;
        while let Some(id) = current {
            let entry = &self.entries[(id.0.get() - 1) as usize];
            if entry.data == data {
                return Ok(Some(id));
            }
            current = entry.next;
        }
        let id = u32::try_from(self.entries.len())
            .ok()
            .and_then(|i| i.checked_add(1))
            .and_then(std::num::NonZeroU32::new)
            .map(DetailsId)
            .ok_or(Error::Limit("expression details limit"))?;
        self.entries.push(Entry { data, next: head });
        self.heads.insert(key, id);
        Ok(Some(id))
    }
}
