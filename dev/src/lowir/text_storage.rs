//! Bounded serialized definition storage; it never transports compiler facts. See NOTICE.
use std::{
    cell::RefCell,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Range,
};

pub const BUFFER_BYTES: usize = 1 << 20;
#[derive(Default)]
pub struct DefinitionBytes {
    buffer: Vec<u8>,
    spill: Option<RefCell<File>>,
    make_spill: Option<fn() -> io::Result<File>>,
    size: usize,
    failed: bool,
}
impl DefinitionBytes {
    #[inline]
    pub fn set_spill_factory(&mut self, factory: fn() -> io::Result<File>) {
        self.make_spill = Some(factory);
    }
    #[cfg(any(test, feature = "test-support"))]
    #[inline]
    pub fn retained_capacity(&self) -> usize {
        self.buffer.capacity()
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    fn flush_buffer(&mut self) -> io::Result<()> {
        if self.spill.is_none() {
            let factory = self
                .make_spill
                .ok_or_else(|| io::Error::other("definition spill factory was not configured"))?;
            self.spill = Some(RefCell::new(factory()?));
        }
        if let Some(file) = &mut self.spill {
            let file = file.get_mut();
            file.seek(SeekFrom::End(0))?;
            file.write_all(&self.buffer)?;
        }
        self.buffer.clear();
        Ok(())
    }
    pub fn copy_range(&self, range: Range<usize>, output: &mut impl Write) -> io::Result<()> {
        if self.failed {
            return Err(io::Error::other("serialized definition spill failed"));
        }
        if range.start > range.end || range.end > self.size {
            return Err(io::Error::other("invalid serialized definition range"));
        }
        let spilled = self.size - self.buffer.len();
        let mut first = range.start;
        if first < spilled {
            let file = self
                .spill
                .as_ref()
                .ok_or_else(|| io::Error::other("missing definition spill file"))?;
            let mut file = file
                .try_borrow_mut()
                .map_err(|_| io::Error::other("definition spill file is already borrowed"))?;
            let offset = u64::try_from(first)
                .map_err(|_| io::Error::other("definition file offset exceeds target"))?;
            file.seek(SeekFrom::Start(offset))?;
            let end = range.end.min(spilled);
            let mut chunk = vec![0; (end - first).min(1 << 16)];
            while first < end {
                let count = chunk.len().min(end - first);
                file.read_exact(&mut chunk[..count])?;
                output.write_all(&chunk[..count])?;
                first += count;
            }
        }
        if first < range.end {
            output.write_all(&self.buffer[first - spilled..range.end - spilled])?;
        }
        Ok(())
    }
}
impl Write for DefinitionBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self
            .size
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("serialized definitions exceed host storage"))?;
        // The frozen DefinitionStream records I/O failure and keeps lowering.
        // A later source error still wins; output reports failure on copy_to.
        if self.failed {
            self.size = end;
            return Ok(bytes.len());
        }
        let mut rest = bytes;
        if !rest.is_empty() && self.buffer.capacity() == 0 {
            self.buffer.reserve_exact(BUFFER_BYTES);
        }
        while !rest.is_empty() {
            let count = rest.len().min(BUFFER_BYTES - self.buffer.len());
            self.buffer.extend_from_slice(&rest[..count]);
            self.size += count;
            rest = &rest[count..];
            if self.buffer.len() == BUFFER_BYTES && self.flush_buffer().is_err() {
                self.failed = true;
                self.buffer.clear();
                self.spill = None;
                self.size = end;
                break;
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
