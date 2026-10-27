//! Pipeline work counters from `opt/optimizer.h` and `optimizer.cpp`. See NOTICE.
use super::pass_context::Context;
#[cfg(feature = "telemetry")]
use crate::lowir::{Error, model::Program};

#[derive(Default)]
pub(crate) struct Statistics {
    pub(super) functions: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub(super) instructions_in: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub(super) instructions_out: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub(super) blocks_in: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    pub(super) blocks_out: crate::support::telemetry::Counter<u64>,
    folded: crate::support::telemetry::Counter<u64>,
    propagated: crate::support::telemetry::Counter<u64>,
    reused: crate::support::telemetry::Counter<u64>,
    dead: crate::support::telemetry::Counter<u64>,
    branches_folded: crate::support::telemetry::Counter<u64>,
    blocks_removed: crate::support::telemetry::Counter<u64>,
    blocks_merged: crate::support::telemetry::Counter<u64>,
    slots_promoted: crate::support::telemetry::Counter<u64>,
    phis_inserted: crate::support::telemetry::Counter<u64>,
    pub(super) calls_inlined: crate::support::telemetry::Counter<u64>,
    pub(super) inlined_instructions: crate::support::telemetry::Counter<u64>,
    pub(super) functions_pruned: crate::support::telemetry::Counter<u64>,
    loads_reused: crate::support::telemetry::Counter<u64>,
    stores_removed: crate::support::telemetry::Counter<u64>,
    loops_removed: crate::support::telemetry::Counter<u64>,
    loops_unrolled: crate::support::telemetry::Counter<u64>,
    pub(super) specialized: crate::support::telemetry::Counter<u64>,
    #[cfg(feature = "telemetry")]
    optimize_ms: crate::support::telemetry::Counter<f64>,
}
impl Statistics {
    pub(super) fn collect(&mut self, context: &Context<'_>) {
        self.folded += context.folded;
        self.propagated += context.propagated;
        self.reused += context.reused;
        self.dead += context.dead;
        self.branches_folded += context.branches_folded;
        self.blocks_removed += context.blocks_removed;
        self.blocks_merged += context.blocks_merged;
        self.slots_promoted += context.slots_promoted;
        self.phis_inserted += context.phis_inserted;
        self.loads_reused += context.loads_reused;
        self.stores_removed += context.stores_removed;
        self.loops_removed += context.loops_removed;
        self.loops_unrolled += context.loops_unrolled;
    }

    #[cfg(feature = "telemetry")]
    pub(super) fn finish(
        &mut self,
        program: &Program,
        start: crate::support::telemetry::Clock,
    ) -> Result<(), Error> {
        #[cfg(feature = "telemetry")]
        {
            (self.instructions_out, self.blocks_out) = size(program)?;
        }

        self.optimize_ms =
            crate::support::telemetry::counter(start.elapsed().as_secs_f64() * 1000.0);
        Ok(())
    }

    #[cfg(feature = "telemetry")]
    pub(crate) fn write(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(
            out,
            "pa32_opt_stats functions={} instructions_in={} instructions_out={} blocks_in={} blocks_out={} folded={} propagated={} reused={} dead={} branches_folded={} blocks_removed={} blocks_merged={} slots_promoted={} phis_inserted={} calls_inlined={} inlined_instructions={} functions_pruned={} loads_reused={} stores_removed={} loops_removed={} loops_unrolled={} specialized={} optimize_ms={}",
            self.functions,
            self.instructions_in,
            self.instructions_out,
            self.blocks_in,
            self.blocks_out,
            self.folded,
            self.propagated,
            self.reused,
            self.dead,
            self.branches_folded,
            self.blocks_removed,
            self.blocks_merged,
            self.slots_promoted,
            self.phis_inserted,
            self.calls_inlined,
            self.inlined_instructions,
            self.functions_pruned,
            self.loads_reused,
            self.stores_removed,
            self.loops_removed,
            self.loops_unrolled,
            self.specialized,
            self.optimize_ms
        )
    }
}

#[cfg(feature = "telemetry")]
pub(super) fn size(program: &Program) -> Result<(u64, u64), Error> {
    let mut instructions = 0_u64;
    let mut blocks = 0_u64;
    for function in program.functions_in_symbol_order() {
        if let Some(body) = &function.body {
            instructions = instructions
                .checked_add(u64::from(super::inline_wave::instruction_count(body)?))
                .ok_or(Error::Limit)?;
            blocks = blocks
                .checked_add(u64::try_from(body.layout.len()).map_err(|_| Error::Limit)?)
                .ok_or(Error::Limit)?;
        }
    }
    Ok((instructions, blocks))
}
