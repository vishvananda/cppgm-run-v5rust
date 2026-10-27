//! Lowering work retained independently of streamed function storage. See NOTICE.
use crate::{
    lowir::model::{GlobalInit, Program, SymbolId},
    sema,
};
use std::{collections::HashSet, io::Write};

#[derive(Default)]
pub(super) struct Work {
    pub(super) units: usize,
    pub(super) mangled_names: usize,
    pub(super) vtables: usize,
    pub(super) lifetime_declarations: HashSet<SymbolId>,
    pub(super) source_reference_storage: HashSet<SymbolId>,
}
pub(crate) struct Statistics {
    units: usize,
    construction: crate::lowir::Construction,
    declarations: usize,
    globals: usize,
    mangled_names: usize,
    vtables: usize,
    semantic: sema::Work,
    symbols: usize,
    names: usize,
    arena_bytes: usize,
}
impl Work {
    pub(super) fn snapshot(&self, program: &Program, semantic: sema::Work) -> Statistics {
        let mut declarations = 0;
        let mut globals = 0;
        for (index, symbol) in program.symbols().iter().enumerate() {
            if program
                .symbol_id(index)
                .is_some_and(|id| self.lifetime_declarations.contains(&id))
            {
                continue;
            }
            if let Some(global) = program.global(symbol) {
                if matches!(global.init, GlobalInit::Declaration) {
                    declarations += 1;
                } else if !program
                    .symbol_id(index)
                    .is_some_and(|id| self.source_reference_storage.contains(&id))
                {
                    globals += 1;
                }
            } else if program.function(symbol).is_some()
                && program
                    .symbol_id(index)
                    .is_some_and(|id| !program.function_defined(id))
            {
                declarations += 1;
            }
        }
        Statistics {
            units: self.units,
            construction: program.construction,
            declarations,
            globals,
            mangled_names: self.mangled_names,
            vtables: self.vtables,
            semantic,
            symbols: program.symbols().len(),
            names: program.names.logical_len(),
            arena_bytes: program.storage_bytes(),
        }
    }
}
impl Statistics {
    pub(crate) fn update_program(&mut self, program: &Program) {
        self.symbols = program.symbols().len();
        self.names = program.names.logical_len();
        self.arena_bytes = program.storage_bytes();
    }
    pub(crate) fn write(&self, out: &mut impl Write) -> std::io::Result<()> {
        let built = self.construction;
        let work = self.semantic;
        write!(
            out,
            "cppgm++ lowir stats: units={} functions={} declarations={} globals={} instructions={} blocks={} slots={} mangled_names={} vtables={} virtual_slots={} override_steps={} specializations={} instantiated_classes={} instantiated_functions={} instantiated_arguments={} instantiated_variables={} value_substitutions={} partial_candidates={} substitution_failures={} limited_lookups={} constant_evaluations={} constant_steps={} base_steps={} symbols={} names={} lowir_arena_bytes={}",
            self.units,
            built.functions,
            self.declarations,
            self.globals,
            built.instructions,
            built.blocks,
            built.slots,
            self.mangled_names,
            self.vtables,
            work.virtual_slots,
            work.override_steps,
            work.specializations,
            work.instantiated_classes,
            work.instantiated_functions,
            work.instantiated_arguments,
            work.instantiated_variables,
            work.value_substitutions,
            work.partial_candidates,
            work.substitution_failures,
            work.limited_lookups,
            work.constant_evaluations,
            work.constant_steps,
            work.base_steps,
            self.symbols,
            self.names,
            self.arena_bytes
        )
    }
}
