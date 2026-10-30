//! Lowering work retained independently of streamed function storage. See NOTICE.
#[cfg(feature = "telemetry")]
use crate::lowir::model::GlobalInit;
use crate::{
    lowir::model::{Program, SymbolId},
    sema,
};
#[cfg(feature = "telemetry")]
use std::io::Write;

#[derive(Default)]
pub(super) struct Work {
    pub(super) units: crate::support::telemetry::Counter<usize>,
    pub(super) mangled_names: crate::support::telemetry::Counter<usize>,
    pub(super) vtables: crate::support::telemetry::Counter<usize>,
    pub(super) lifetime_declarations: crate::support::telemetry::Set<SymbolId>,
    pub(super) source_reference_storage: crate::support::telemetry::Set<SymbolId>,
}
#[derive(Default)]
pub struct Statistics {
    #[cfg(feature = "telemetry")]
    units: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    construction: crate::lowir::Construction,
    #[cfg(feature = "telemetry")]
    declarations: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    globals: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    mangled_names: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    vtables: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    semantic: sema::Work,
    #[cfg(feature = "telemetry")]
    symbols: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    names: crate::support::telemetry::Counter<usize>,
    #[cfg(feature = "telemetry")]
    arena_bytes: crate::support::telemetry::Counter<usize>,
}
impl Work {
    #[cfg(feature = "telemetry")]
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
    #[cfg(not(feature = "telemetry"))]
    pub(super) fn snapshot(&self, program: &Program, semantic: sema::Work) -> Statistics {
        let _ = self;
        let _ = (program, semantic);
        Statistics::default()
    }
}
impl Statistics {
    #[cfg(feature = "telemetry")]
    pub fn update_program(&mut self, program: &Program) {
        self.symbols = program.symbols().len();
        self.names = program.names.logical_len();
        self.arena_bytes = program.storage_bytes();
    }
    #[cfg(not(feature = "telemetry"))]
    pub fn update_program(&mut self, program: &Program) {
        let _ = (self, program);
    }
    #[cfg(feature = "telemetry")]
    pub fn write(&self, out: &mut impl Write) -> std::io::Result<()> {
        let built = self.construction;
        let work = self.semantic;
        write!(
            out,
            "cppgm++ lowir stats: units={} functions={} declarations={} globals={} instructions={} blocks={} slots={} mangled_names={} vtables={} virtual_slots={} override_steps={} specializations={} instantiated_classes={} instantiated_functions={} instantiated_arguments={} instantiated_variables={} value_substitutions={} partial_candidates={} substitution_failures={} limited_lookups={} constant_evaluations={} constant_steps={} base_steps={} constant_call_builds={} constant_call_reuses={} function_order_builds={} function_order_reuses={} symbols={} names={} lowir_arena_bytes={}",
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
            work.constant_call_builds,
            work.constant_call_reuses,
            work.function_order_builds,
            work.function_order_reuses,
            self.symbols,
            self.names,
            self.arena_bytes
        )
    }
}
