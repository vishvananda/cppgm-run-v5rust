// Translated from symbol_lowering.cpp's explicit and implicit ABI tags. See NOTICE.
use super::Symbols;
use crate::{
    abi::graph::{Node, NodeId},
    lower::Error,
    sema::lowering::{EntityId, TypeId},
};

impl Symbols<'_, '_> {
    pub(super) fn tagged_entity_name(
        &mut self,
        name: Node,
        entity: Option<EntityId>,
    ) -> Result<NodeId, Error> {
        let tags = entity.map_or(&[][..], |entity| self.graph.lower_abi_tags(entity));
        let mut nodes = Vec::with_capacity(tags.len());
        for tag in tags {
            nodes.push(self.abi.string(self.cursor.name(*tag))?);
        }
        Ok(self.abi.intern(name, &nodes)?)
    }

    pub(super) fn with_implicit_tags(
        &mut self,
        entity: NodeId,
        ty: TypeId,
    ) -> Result<NodeId, Error> {
        if !self.graph.lower_has_abi_tags() {
            return Ok(entity);
        }
        let ty = self.abi_type(ty)?;
        let mut tags = self.abi.collect_tags(ty);
        if tags.is_empty() {
            return Ok(entity);
        }
        let used = self.abi.collect_tags(entity);
        tags.retain(|id| {
            used.binary_search_by_key(&id.index(), |id| id.index())
                .is_err()
        });
        if tags.is_empty() {
            return Ok(entity);
        }
        let derived = if let Node::Function {
            name,
            result,
            qualifiers,
            variadic,
        } = self.abi.get(entity)
        {
            let name = self.abi.tagged(name, &tags)?;
            let parameters = self.abi.items(entity).to_vec();
            self.abi.intern(
                Node::Function {
                    name,
                    result,
                    qualifiers,
                    variadic,
                },
                &parameters,
            )?
        } else {
            self.abi.tagged(entity, &tags)?
        };
        self.abi.note_implicit_tags(derived, entity);
        Ok(derived)
    }
}
