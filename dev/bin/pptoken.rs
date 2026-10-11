use cppgm_rust::{
    driver::pptoken::{PpTokenError, emit_pptokens},
    preprocess::source::SourceBuffer,
};
use std::fs::File;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::os::fd::AsFd;
use std::process::ExitCode;

// The frozen CLI uses iostreams with exceptions disabled: an output failure
// sticks, but cannot stop lexing or replace a later source diagnostic. Keep
// this policy here; the shared rendering API still reports writer failures.
struct OutputStream<W> {
    writer: W,
    failed: bool,
}

impl<W: Write> Write for OutputStream<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.failed && self.writer.write_all(bytes).is_err() {
            self.failed = true;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.failed && self.writer.flush().is_err() {
            self.failed = true;
        }
        Ok(())
    }
}

fn read_source(input: &mut (impl Read + Seek)) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    // SourceBuffer::ReadFrom probes both positions, then rewinds and reserves
    // one whole image for seekable input. Pipes simply fail the seek probes.
    let start = input.stream_position();
    let end = input.seek(SeekFrom::End(0));
    if let (Ok(start), Ok(end)) = (start, end)
        && end >= start
    {
        if input.seek(SeekFrom::Start(start)).ok() != Some(start) {
            return Err("cannot rewind source input".to_owned());
        }
        let capacity = usize::try_from(end - start)
            .ok()
            .and_then(|size| size.checked_add(1))
            .filter(|&size| isize::try_from(size).is_ok())
            .ok_or_else(|| "basic_string::_M_create".to_owned())?;
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| "std::bad_alloc".to_owned())?;
    }
    // C++ streambuf stops on a read error and keeps the bytes already read.
    let _read_status = input.read_to_end(&mut bytes);
    Ok(bytes)
}

fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = OutputStream {
        writer: io::BufWriter::new(stdout.lock()),
        failed: false,
    };
    if std::env::args_os()
        .skip(1)
        .any(|arg| arg == "--batch-stdin")
    {
        for line in stdin.lock().split(b'\n') {
            if line.is_err() {
                break;
            }
            writeln!(output, "EXIT_NOT_IMPLEMENTED").map_err(|e| e.to_string())?;
            output.flush().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    // A safe duplicate retains stdin's current file position without owning
    // the standard descriptor. Failed reads of a closed stream yield EOF.
    let bytes = if let Ok(descriptor) = stdin.as_fd().try_clone_to_owned() {
        read_source(&mut File::from(descriptor))?
    } else {
        Vec::new()
    };
    let source = SourceBuffer::new(bytes).map_err(str::to_owned)?;
    let result = emit_pptokens(&source, &mut output);
    output.flush().map_err(|e| e.to_string())?;
    match result {
        Ok(()) => Ok(()),
        Err(PpTokenError::Source(error)) => Err(format!("<stdin>:{}", source.format_error(&error))),
        Err(PpTokenError::Output(error)) => Err(error.to_string()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            // cerr also leaves a failed stream silent, rather than panicking.
            let _write_status = writeln!(io::stderr().lock(), "ERROR: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_output_keeps_scanning_until_the_source_error() {
        let source = SourceBuffer::new(b"ok\n\"\\q\"".to_vec()).unwrap();
        let mut storage = [0u8; 1];
        let mut output = OutputStream {
            writer: io::Cursor::new(&mut storage[..]),
            failed: false,
        };
        let Err(PpTokenError::Source(error)) = emit_pptokens(&source, &mut output) else {
            panic!("expected source diagnostic even after the output failed");
        };
        assert!(output.failed);
        assert_eq!(source.format_error(&error), "2:2: invalid escape sequence");
        output.flush().unwrap();
    }

    #[test]
    fn seekable_input_retains_the_current_position() {
        let mut input = io::Cursor::new(b"ignored kept".as_slice());
        input.set_position(8);
        assert_eq!(read_source(&mut input).unwrap(), b"kept");
    }
}
