//! Linker observations (`native/link/linker.cpp`). See NOTICE.
use std::io::Write;
#[derive(Default)]
pub(crate) struct Statistics {
    pub(super) objects: usize,
    pub(super) sections: usize,
    pub(super) discarded_groups: usize,
    pub(super) symbols: usize,
    pub(super) relocations: usize,
    pub(super) got_slots: usize,
    pub(super) runtime_names: usize,
    pub(super) text_bytes: usize,
    pub(super) rodata_bytes: usize,
    pub(super) data_bytes: usize,
    pub(super) bss_bytes: u64,
    pub(super) runtime_ms: f64,
    pub(super) link_ms: f64,
}
impl Statistics {
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
