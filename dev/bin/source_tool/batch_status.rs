// Translated from the tools' RunNotImplementedBatchMode. See NOTICE.
//! A build without the linked test runner reports unavailable request records.
use std::io::{BufRead, Write};

pub(super) fn write(input: &mut impl BufRead, output: &mut impl Write) {
    let mut line = Vec::new();
    while matches!(input.read_until(b'\n', &mut line), Ok(1..)) {
        let _status = writeln!(output, "EXIT_NOT_IMPLEMENTED");
        let _status = output.flush();
        line.clear();
    }
}
