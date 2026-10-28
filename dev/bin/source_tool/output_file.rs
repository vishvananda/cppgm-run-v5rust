// Translated from lowir/text_writer.cpp::write_program_file. See NOTICE.
//! A destination retains ownership through write failure and one checked close.
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

pub(super) struct OutputFile {
    file: BufWriter<File>,
    failed: bool,
}
impl OutputFile {
    pub(super) fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            file: BufWriter::new(File::create(path)?),
            failed: false,
        })
    }
    pub(super) fn finish(mut self) -> bool {
        // An earlier serialization failure still flushes healthy pending output.
        // Failed writes are sticky and must never be retried during extraction.
        let _ = self.flush();
        let (file, pending) = self.file.into_parts();
        let closed = nix::unistd::close(file).is_ok();
        !self.failed && pending.is_ok() && closed
    }
}
impl Write for OutputFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.failed && self.file.write_all(bytes).is_err() {
            self.failed = true;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if !self.failed && self.file.flush().is_err() {
            self.failed = true;
        }
        Ok(())
    }
}
