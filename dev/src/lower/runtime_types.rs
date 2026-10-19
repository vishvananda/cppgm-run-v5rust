// Translated from lower/rtti_lowering.cpp::runtime_function. See NOTICE.
//! Program-owned ABI entry identities are declared only when a cast needs them.
use super::{Error, Symbols};
use crate::lowir::{
    model::{
        Boundary, Function, Linkage, Metadata, Parameter, Program, Returns, Signature, SymbolId,
    },
    vocabulary::{Binding, Role, Type},
};
use crate::sema::lowering::EntityId;
use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
struct Hint {
    count: u32,
    virtual_path: bool,
    offset: u64,
}
impl Hint {
    fn value(self) -> i64 {
        if self.count == 0 {
            -2
        } else if self.count == 1 && !self.virtual_path {
            i64::from_le_bytes(self.offset.to_le_bytes())
        } else if self.virtual_path {
            -1
        } else {
            -3
        }
    }
}
#[derive(Default)]
pub(super) struct CastHints {
    // Layouts and base edges are frozen before lowering. Facts live for this unit.
    pairs: HashMap<(EntityId, EntityId), i64>,
    current: Option<EntityId>,
    grouped: bool,
    // Like the C++ owner, keep at most one derived class's walk facts.
    facts: Option<HashMap<EntityId, Hint>>,
}

fn walk_hints(
    graph: &crate::sema::Graph,
    derived: EntityId,
    base: Option<EntityId>,
) -> Result<Option<HashMap<EntityId, Hint>>, Error> {
    let mut work = vec![(derived, 0_u64, true, false)];
    let mut facts = HashMap::<EntityId, Hint>::new();
    let mut visits = 0_u32;
    while let Some((class, offset, public, virtual_path)) = work.pop() {
        visits += 1;
        if visits > 1 << 16 {
            return Ok(None);
        }
        if public && (base.is_none() || base == Some(class)) {
            let fact = facts.entry(class).or_default();
            fact.count += 1;
            fact.virtual_path |= virtual_path;
            fact.offset = offset;
        }
        if base == Some(class) {
            continue;
        }
        for edge in graph.lower_cast_bases(class) {
            let (next, accessible, shared, delta) = edge?;
            work.push((
                next,
                offset.wrapping_add(delta),
                public && accessible,
                virtual_path || shared,
            ));
        }
    }
    Ok(Some(facts))
}

impl Symbols<'_, '_> {
    pub(in crate::lower) fn dynamic_cast_hint(
        &mut self,
        base: EntityId,
        derived: EntityId,
    ) -> Result<i64, Error> {
        if let Some(&hint) = self.cast_hints.pairs.get(&(derived, base)) {
            return Ok(hint);
        }
        if self.cast_hints.current == Some(derived) && !self.cast_hints.grouped {
            self.cast_hints.facts = walk_hints(self.graph, derived, None)?;
            self.cast_hints.grouped = true;
        }
        let hint = if self.cast_hints.current == Some(derived)
            && let Some(facts) = self.cast_hints.facts.as_ref()
        {
            facts.get(&base).copied().unwrap_or_default().value()
        } else {
            if self.cast_hints.current != Some(derived) {
                self.cast_hints.current = Some(derived);
                self.cast_hints.grouped = false;
                self.cast_hints.facts = None;
            }
            walk_hints(self.graph, derived, Some(base))?.map_or(-1, |facts| {
                facts.get(&base).copied().unwrap_or_default().value()
            })
        };
        self.cast_hints.pairs.insert((derived, base), hint);
        Ok(hint)
    }
    pub(in crate::lower) fn cast_runtime(
        &mut self,
        program: &mut Program,
        failure: bool,
    ) -> Result<SymbolId, Error> {
        self.rtti_runtime(
            program,
            if failure {
                Role::BadCast
            } else {
                Role::DynamicCast
            },
        )
    }
    pub(in crate::lower) fn rtti_runtime(
        &mut self,
        program: &mut Program,
        requested: Role,
    ) -> Result<SymbolId, Error> {
        let failure = requested != Role::DynamicCast;
        let known = if requested == Role::BadTypeId {
            self.table.bad_typeid
        } else if failure {
            self.table.bad_cast
        } else {
            self.table.dynamic_cast
        };
        if let Some(symbol) = known {
            return Ok(symbol);
        }
        let (object, role, types, result): (_, _, &[Type], _) = if requested == Role::BadTypeId {
            ("__cxa_bad_typeid", Role::BadTypeId, &[], Type::Void)
        } else if failure {
            ("__cxa_bad_cast", Role::BadCast, &[], Type::Void)
        } else {
            (
                "__dynamic_cast",
                Role::DynamicCast,
                &[Type::Ptr, Type::Ptr, Type::Ptr, Type::I64],
                Type::Ptr,
            )
        };
        let symbol = self.table.allocate(
            program,
            &format!("__external_runtime__{object}"),
            None,
            true,
        )?;
        let parameters = types
            .iter()
            .enumerate()
            .map(|(index, &ty)| {
                Ok(Parameter {
                    name: program.names.intern(&format!("arg{index}"))?,
                    ty,
                    pass: None,
                    object_bytes: 0,
                    no_alias: false,
                    classes: None,
                })
            })
            .collect::<Result<_, crate::lowir::Error>>()?;
        let signature = program.add_signature(Signature {
            parameters,
            result,
            boundary: Boundary {
                returns: if failure {
                    Returns::NoReturn
                } else {
                    Returns::Default
                },
                ..Boundary::default()
            },
        })?;
        let object = program.names.intern(object)?;
        program.add(
            symbol,
            Function {
                name: program.symbol(symbol).name,
                signature,
                metadata: Metadata {
                    role: Some(role),
                    linkage: Linkage::C,
                    binding: Some(Binding::Strong),
                    object: Some(object),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        program.publish_runtime_declaration(symbol);
        self.table
            .runtime_roles
            .push(crate::lower::runtime_roles::RoleSymbol {
                symbol,
                role,
                origin: crate::lower::runtime_roles::Origin::Runtime,
            });
        if requested == Role::BadTypeId {
            self.table.bad_typeid = Some(symbol);
        } else if failure {
            self.table.bad_cast = Some(symbol);
        } else {
            self.table.dynamic_cast = Some(symbol);
        }
        Ok(symbol)
    }
}
