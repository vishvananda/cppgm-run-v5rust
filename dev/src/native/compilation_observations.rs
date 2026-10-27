//! Object compilation observations, retained after each function is released. See NOTICE.
#[cfg(feature = "telemetry")]
use super::object::Object;
use super::{Statistics, mir};
use crate::lowir::Error;
#[cfg(feature = "telemetry")]
use std::io::Write;

#[derive(Clone, Copy, Default)]
pub(crate) struct ObservationOptions {
    pub(crate) census: bool,
    pub(crate) clocks: bool,
}
#[derive(Default)]
pub(crate) struct Observations {
    pub(crate) census: Vec<String>,
    pub(crate) statistics: ObjectStatistics,
}
#[derive(Default)]
pub(crate) struct ObjectStatistics {
    backend: Statistics,
    pub(super) landings: crate::support::telemetry::Counter<usize>,
    pub(super) lsdas: crate::support::telemetry::Counter<usize>,
    pub(super) unwind_bytes: crate::support::telemetry::Counter<usize>,
    pub(super) pruned_functions: crate::support::telemetry::Counter<usize>,
    eh_walks: crate::support::telemetry::Counter<usize>,
    eh_states: crate::support::telemetry::Counter<usize>,
}
impl ObjectStatistics {
    pub(super) fn selected(
        &mut self,
        function: &mir::Function,
        milliseconds: f64,
    ) -> Result<(), Error> {
        self.backend.selected(function)?;
        self.backend.select_ms += milliseconds;
        self.eh_walks += function.eh.walks;
        self.eh_states += function.eh.states;
        Ok(())
    }
    pub(super) fn encoded(
        &mut self,
        emission: mir::Emission,
        milliseconds: f64,
    ) -> Result<(), Error> {
        self.backend.encoded(emission)?;
        self.backend.encode_ms += milliseconds;
        Ok(())
    }
    #[cfg(feature = "telemetry")]
    pub(crate) fn write(&self, object: &Object, out: &mut impl Write) -> std::io::Result<()> {
        let backend = &self.backend;
        write!(
            out,
            "functions={} lowir_instructions={} mir_instructions={} code_bytes={} sections={} symbols={} relocations={} landings={} lsdas={} unwind_bytes={} eh_walks={} eh_states={} pruned_functions={}",
            backend.functions,
            backend.lowir_instructions,
            backend.mir_instructions,
            backend.code_bytes,
            object.section_count() - 1,
            object.symbol_count() - 1,
            object
                .sections()
                .map(|(_, section)| section.relocations.len())
                .sum::<usize>(),
            self.landings,
            self.lsdas,
            self.unwind_bytes,
            self.eh_walks,
            self.eh_states,
            self.pruned_functions
        )
    }

    #[cfg(feature = "telemetry")]
    pub(crate) fn write_clocks(&self, out: &mut impl Write) -> std::io::Result<()> {
        write!(
            out,
            " select_ms={} encode_ms={}",
            self.backend.select_ms, self.backend.encode_ms
        )
    }
}
