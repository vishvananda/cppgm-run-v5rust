// Translated from the tools' RunNotImplementedBatchMode. See NOTICE.
//! Transport for builds without a linked test runner; it executes no compiler phase.
use std::io::{BufRead, Write};

/// Consume byte request lines and report the unavailable runner, retaining the
/// frozen stream's sticky output failure and successful input-failure status.
pub fn write(input: &mut impl BufRead, output: &mut impl Write) {
    let mut line = Vec::new();
    let mut failed = false;
    while matches!(input.read_until(b'\n', &mut line), Ok(1..)) {
        if !failed {
            failed = writeln!(output, "EXIT_NOT_IMPLEMENTED").is_err() || output.flush().is_err();
        }
        line.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct RecoveringWriter(usize);
    impl Write for RecoveringWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += 1;
            if self.0 == 1 {
                Err(std::io::Error::other("first write failed"))
            } else {
                Ok(bytes.len())
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn failed_output_stays_failed_while_input_records_are_consumed() {
        let records = b"first\n\xff\n\nlast\0";
        let mut input = std::io::Cursor::new(records);
        let mut output = RecoveringWriter(0);
        write(&mut input, &mut output);
        assert_eq!(input.position(), u64::try_from(records.len()).unwrap());
        assert_eq!(output.0, 1);
    }
}
