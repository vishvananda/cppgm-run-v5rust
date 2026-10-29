// Translated from lower/symbol_lowering.cpp::state_pending_signatures and
// lower/program_lowering.cpp::state_forward_boundaries. See NOTICE.
//! Referenced declarations retain semantic identity only until their unit ends.
use super::{Error, Metadata, Program, SymbolId};
use crate::lower::{construction::Entry, function, unit_frontend::Context};
use crate::lowir::model::{InstructionKind, SignatureId, Value};
use crate::support::hash::HashMap;

#[derive(Clone, Copy)]
struct Declaration {
    symbol: SymbolId,
    entry: Entry,
    metadata: Metadata,
    signature: Option<SignatureId>,
}
#[derive(Default)]
pub(super) struct Declarations {
    ordered: Vec<Declaration>,
    index: HashMap<SymbolId, usize>,
}
impl Declarations {
    pub(super) fn reserve(&mut self, symbol: SymbolId, entry: Entry, metadata: Metadata) {
        if self.index.contains_key(&symbol) {
            return;
        }
        self.index.insert(symbol, self.ordered.len());
        self.ordered.push(Declaration {
            symbol,
            entry,
            metadata,
            signature: None,
        });
    }
}

impl Context<'_, '_, '_> {
    pub(in crate::lower) fn function_nothrow(
        &mut self,
        entity: crate::sema::lowering::EntityId,
    ) -> Result<bool, Error> {
        Ok(self.graph.lower_demand_function_nothrow(
            self.tree,
            self.cursor,
            self.symbols.budget,
            self.symbols.stack_base,
            entity,
        )?)
    }

    pub(in crate::lower) fn declaration_signature(
        &mut self,
        program: &mut Program,
        entry: Entry,
    ) -> Result<SignatureId, Error> {
        let mut signature = function::declaration_shape(
            program,
            &mut self.symbols.view(&*self.graph, &*self.tree, self.cursor),
            &*self.graph,
            entry,
        )?;
        let nothrow = if let Some(entity) = entry.entity(&*self.graph) {
            self.function_nothrow(entity)?
        } else {
            entry.facts(&*self.graph)?.nothrow
        };
        signature.boundary.unwind = if nothrow {
            crate::lowir::model::Unwind::No
        } else {
            crate::lowir::model::Unwind::Default
        };
        let facts = entry.facts(&*self.graph)?;
        function::builtin_boundary(&mut signature.boundary, facts.builtin);
        function::declared_effects(
            &mut signature.boundary,
            &*self.graph,
            entry.entity(&*self.graph),
        );
        Ok(program.add_signature(signature)?)
    }

    fn complete_declaration(
        &mut self,
        program: &mut Program,
        index: usize,
    ) -> Result<SignatureId, Error> {
        let pending = self.symbols.declarations.ordered[index];
        if let Some(signature) = pending.signature {
            return Ok(signature);
        }
        let signature = self.declaration_signature(program, pending.entry)?;
        self.symbols.declarations.ordered[index].signature = Some(signature);
        program.set_forward_boundary(pending.symbol, signature, pending.metadata);
        Ok(signature)
    }

    pub(in crate::lower) fn prepare_forward_boundaries(
        &mut self,
        program: &mut Program,
    ) -> Result<(), Error> {
        if !program.needs_forward_boundaries() {
            return Ok(());
        }
        let Some(body) = program
            .completed_definition()
            .and_then(|symbol| program.function(program.symbol(symbol)))
            .and_then(|function| function.body.as_ref())
        else {
            return Ok(());
        };
        // Freeze instruction order before a frontend demand mutates its graph.
        let callees: Vec<_> = body
            .instructions()
            .iter()
            .filter_map(|instruction| match &instruction.kind {
                InstructionKind::Call(call) if call.signature.is_none() => {
                    if let Value::Symbol(symbol) = call.callee {
                        Some(symbol)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();
        for callee in callees {
            if program.symbol(callee).declared() || program.forward_boundary(callee).is_some() {
                continue;
            }
            if let Some(&index) = self.symbols.declarations.index.get(&callee) {
                self.complete_declaration(program, index)?;
            }
        }
        Ok(())
    }

    pub(in crate::lower) fn finish_function_declarations(
        &mut self,
        program: &mut Program,
    ) -> Result<(), Error> {
        let mut index = 0;
        while index < self.symbols.declarations.ordered.len() {
            let pending = self.symbols.declarations.ordered[index];
            if !program.symbol(pending.symbol).declared() {
                let signature = self.complete_declaration(program, index)?;
                let mut metadata = pending.metadata;
                if matches!(
                    pending.entry,
                    Entry::Constructor(_, _) | Entry::DestructorBase(_)
                ) {
                    metadata.flags &= !4;
                }
                program.add(
                    pending.symbol,
                    crate::lowir::model::Function {
                        name: program.symbol(pending.symbol).name,
                        signature,
                        metadata,
                        body: None,
                        location: None,
                    },
                )?;
            }
            index += 1;
        }
        // No semantic IDs survive in the program-wide symbol table.
        self.symbols.declarations = Declarations::default();
        Ok(())
    }

    pub(in crate::lower) fn stream_definition(
        &mut self,
        program: &mut Program,
        output: &mut crate::lower::definition_order::Definitions,
        entry: Entry,
    ) -> Result<(), Error> {
        self.prepare_forward_boundaries(program)?;
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .stream_definition(program, output, entry)
    }
}
