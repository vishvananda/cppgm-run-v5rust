use cppgm_rust::{
    driver::pptoken::{PpTokenError, emit_pptokens},
    preprocess::source::SourceBuffer,
};
use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    if std::env::args_os()
        .skip(1)
        .any(|arg| arg == "--batch-stdin")
    {
        for line in stdin.lock().split(b'\n') {
            line.map_err(|e| e.to_string())?;
            writeln!(output, "EXIT_NOT_IMPLEMENTED").map_err(|e| e.to_string())?;
            output.flush().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let mut bytes = Vec::new();
    stdin
        .lock()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
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
            eprintln!("ERROR: {message}");
            ExitCode::FAILURE
        }
    }
}
