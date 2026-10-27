// Translated from lower/rtti_lowering.cpp::runtime_function. See NOTICE.
//! Program-owned ABI entries are declared once when a runtime operation demands them.
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
        if let Some(&hint) = self.state.cast_hints.pairs.get(&(derived, base)) {
            return Ok(hint);
        }
        if self.state.cast_hints.current == Some(derived) && !self.state.cast_hints.grouped {
            self.state.cast_hints.facts = walk_hints(self.graph, derived, None)?;
            self.state.cast_hints.grouped = true;
        }
        let hint = if self.state.cast_hints.current == Some(derived)
            && let Some(facts) = self.state.cast_hints.facts.as_ref()
        {
            facts.get(&base).copied().unwrap_or_default().value()
        } else {
            if self.state.cast_hints.current != Some(derived) {
                self.state.cast_hints.current = Some(derived);
                self.state.cast_hints.grouped = false;
                self.state.cast_hints.facts = None;
            }
            walk_hints(self.graph, derived, Some(base))?.map_or(-1, |facts| {
                facts.get(&base).copied().unwrap_or_default().value()
            })
        };
        self.state.cast_hints.pairs.insert((derived, base), hint);
        Ok(hint)
    }
    pub(in crate::lower) fn cast_runtime(
        &mut self,
        program: &mut Program,
        failure: bool,
    ) -> Result<SymbolId, Error> {
        self.runtime_function(
            program,
            if failure {
                Role::BadCast
            } else {
                Role::DynamicCast
            },
        )
    }
    pub(in crate::lower) fn runtime_function(
        &mut self,
        program: &mut Program,
        requested: Role,
    ) -> Result<SymbolId, Error> {
        if let Some(&symbol) = self.state.table.runtime_entries.get(&requested) {
            return Ok(symbol);
        }
        let (object, types, result, noreturn): (_, &[Type], _, _) = match requested {
            Role::BadTypeId => ("__cxa_bad_typeid", &[], Type::Void, true),
            Role::BadCast => ("__cxa_bad_cast", &[], Type::Void, true),
            Role::DynamicCast => (
                "__dynamic_cast",
                &[Type::Ptr, Type::Ptr, Type::Ptr, Type::I64],
                Type::Ptr,
                false,
            ),
            Role::EhAllocateException => {
                ("__cxa_allocate_exception", &[Type::I64], Type::Ptr, false)
            }
            Role::EhThrow => (
                "__cxa_throw",
                &[Type::Ptr, Type::Ptr, Type::Ptr],
                Type::Void,
                true,
            ),
            Role::EhRethrow => ("__cxa_rethrow", &[], Type::Void, true),
            Role::EhBeginCatch => ("__cxa_begin_catch", &[Type::Ptr], Type::Ptr, false),
            Role::EhEndCatch => ("__cxa_end_catch", &[], Type::Void, false),
            _ => {
                return Err(
                    crate::sema::Error::Invariant("unsupported exception runtime entry").into(),
                );
            }
        };
        let symbol = self.declare_runtime_function(
            program,
            object,
            types,
            result,
            noreturn,
            Some(requested),
        )?;
        self.state.table.runtime_entries.insert(requested, symbol);
        Ok(symbol)
    }
    pub(in crate::lower) fn unexpected_function(
        &mut self,
        program: &mut Program,
    ) -> Result<SymbolId, Error> {
        if let Some(symbol) = self.state.table.unexpected_function {
            return Ok(symbol);
        }
        let symbol = self.declare_runtime_function(
            program,
            "__cxa_call_unexpected",
            &[Type::Ptr],
            Type::Void,
            true,
            None,
        )?;
        self.state.table.unexpected_function = Some(symbol);
        Ok(symbol)
    }
    fn declare_runtime_function(
        &mut self,
        program: &mut Program,
        object: &str,
        types: &[Type],
        result: Type,
        noreturn: bool,
        role: Option<Role>,
    ) -> Result<SymbolId, Error> {
        let symbol = self.state.table.allocate(
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
                returns: if noreturn {
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
                    role,
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
        if let Some(role) = role {
            self.state
                .table
                .runtime_roles
                .push(crate::lower::runtime_roles::RoleSymbol {
                    symbol,
                    role,
                    origin: crate::lower::runtime_roles::Origin::Runtime,
                });
        }
        Ok(symbol)
    }
}
