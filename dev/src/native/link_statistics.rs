//! Linker observations (`native/link/linker.cpp`). See NOTICE.
#[cfg(feature = "telemetry")]
use std::io::Write;
#[derive(Default)]
pub(crate) struct Statistics {
    pub(super) objects: crate::support::telemetry::Counter<usize>,
    pub(super) sections: crate::support::telemetry::Counter<usize>,
    pub(super) discarded_groups: crate::support::telemetry::Counter<usize>,
    pub(super) symbols: crate::support::telemetry::Counter<usize>,
    pub(super) relocations: crate::support::telemetry::Counter<usize>,
    pub(super) got_slots: crate::support::telemetry::Counter<usize>,
    pub(super) runtime_names: crate::support::telemetry::Counter<usize>,
    pub(super) text_bytes: crate::support::telemetry::Counter<usize>,
    pub(super) rodata_bytes: crate::support::telemetry::Counter<usize>,
    pub(super) data_bytes: crate::support::telemetry::Counter<usize>,
    pub(super) bss_bytes: crate::support::telemetry::Counter<u64>,
    pub(super) runtime_ms: crate::support::telemetry::Counter<f64>,
    pub(super) link_ms: crate::support::telemetry::Counter<f64>,
}
impl Statistics {
    #[cfg(feature = "telemetry")]
    pub(crate) fn write(&self, out: &mut impl Write, peak_rss: u64) -> std::io::Result<()> {
        writeln!(
            out,
            "cppgm++ link stats: objects={} sections={} discarded_groups={} symbols={} relocations={} got_slots={} runtime_names={} text_bytes={} rodata_bytes={} data_bytes={} bss_bytes={} runtime_ms={} link_ms={} peak_rss_kb={peak_rss}",
            self.objects,
            self.sections,
            self.discarded_groups,
            self.symbols,
            self.relocations,
            self.got_slots,
            self.runtime_names,
            self.text_bytes,
            self.rodata_bytes,
            self.data_bytes,
            self.bss_bytes,
            self.runtime_ms,
            self.link_ms
        )
    }
}
