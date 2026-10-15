// Translated from lower/symbol_lowering.cpp::unnamed_ordinal/type_unqualified_name
// and local_mangling.cpp::anonymous_name. See NOTICE.
//! Unit-owned ABI identities number a class's unnamed member types once.
use super::Error;
use crate::{
    abi::graph::{Graph as AbiGraph, Node, NodeId},
    sema::{
        Graph,
        lowering::{EntityId, ScopeId},
    },
};
use std::collections::HashMap;
#[derive(Default)]
pub(super) struct Names {
    numbered: HashMap<EntityId, u32>,
    private: HashMap<EntityId, u32>,
    next_private: u32,
}
impl Names {
    pub(super) fn scope_terminal(
        &mut self,
        graph: &Graph,
        abi: &mut AbiGraph,
        scope: ScopeId,
    ) -> Result<Option<NodeId>, Error> {
        let Some((class, _, numbered)) = graph.lower_anonymous_scope(scope) else {
            return Ok(None);
        };
        if numbered {
            if !self.numbered.contains_key(&class) {
                let home = graph.lower_presentation_scope(scope).0.ok_or(
                    crate::sema::Error::Invariant("anonymous class without owner"),
                )?;
                let mut next = 0u32;
                for entity in graph.lower_unnamed_scope_types(home) {
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        self.numbered.entry(entity)
                    {
                        entry.insert(next);
                        next = next.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
                    }
                }
            }
            let ordinal = *self
                .numbered
                .get(&class)
                .ok_or(crate::sema::Error::Invariant(
                    "unnamed type missing from declarations",
                ))?;
            return Ok(Some(abi.intern(Node::UnnamedType(ordinal), &[])?));
        }
        let ordinal = match self.private.entry(class) {
            std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let ordinal = self.next_private;
                self.next_private = ordinal.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
                entry.insert(ordinal);
                ordinal
            }
        };
        Ok(Some(abi.string(format!("$_{ordinal}").as_bytes())?))
    }
}
