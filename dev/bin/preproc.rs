// Translated from dev/preproc.cpp. See NOTICE.
use cppgm_rust::{
    driver::preproc::{PreprocToolError, describe, emit_preprocessed},
    preprocess::error::PreprocessError,
    support::source::SourceBuffer,
};
use std::{
    fs::File,
    io::{self, BufRead, Read, Write},
    process::ExitCode,
};

// Like the frozen ofstream, retain output failure while continuing source work.
struct OutputStream {
    file: Option<io::BufWriter<File>>,
}
impl Write for OutputStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .file
            .as_mut()
            .is_some_and(|file| file.write_all(bytes).is_err())
        {
            self.file = None;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.file.as_mut().is_some_and(|file| file.flush().is_err()) {
            self.file = None;
        }
        Ok(())
    }
}
enum CliError {
    Message(String),
    Diagnostic(Vec<u8>),
    Missing(&'static str),
    Internal(&'static str),
}
fn run() -> Result<(), CliError> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--batch-stdin") {
        let mut output = io::stdout().lock();
        for line in io::stdin().lock().lines() {
            if line.is_err() {
                break;
            }
            let _status = writeln!(output, "EXIT_NOT_IMPLEMENTED");
            let _status = output.flush();
        }
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "--stats") {
        return Err(CliError::Missing("preprocessing counters"));
    }
    if args.len() < 3 || args[0] != "-o" {
        return Err(CliError::Message("invalid usage".to_owned()));
    }
    let mut output = OutputStream {
        file: File::create(&args[1]).ok().map(io::BufWriter::new),
    };
    writeln!(output, "preproc {}", args.len() - 2)
        .map_err(|error| CliError::Message(error.to_string()))?;
    for path in &args[2..] {
        writeln!(output, "sof {path}").map_err(|error| CliError::Message(error.to_string()))?;
        let mut bytes = Vec::new();
        let mut file = File::open(path)
            .map_err(|_| CliError::Message(format!("{path}: cannot open the source file")))?;
        let _read_status = file.read_to_end(&mut bytes);
        let source =
            SourceBuffer::new(bytes).map_err(|message| CliError::Message(message.to_owned()))?;
        emit_preprocessed(&source, path.as_bytes(), &mut output).map_err(|error| match error {
            PreprocToolError::Initialization(PreprocessError::Source(error)) => {
                CliError::Message(error.to_string())
            }
            PreprocToolError::Initialization(error) => match error {
                PreprocessError::NotImplemented(group) => CliError::Missing(group),
                PreprocessError::Invariant(message) => CliError::Internal(message),
                _ => CliError::Message(format!("{path}:{}", describe(&source, &error))),
            },
            PreprocToolError::Preprocess { error, diagnostic } => match error {
                PreprocessError::NotImplemented(group) => CliError::Missing(group),
                PreprocessError::Invariant(message) => CliError::Internal(message),
                _ => CliError::Diagnostic(diagnostic),
            },
            PreprocToolError::InvalidToken => {
                CliError::Message(format!("{path}: invalid phase-7 token"))
            }
            PreprocToolError::Output(error) => CliError::Message(error.to_string()),
        })?;
    }
    output
        .flush()
        .map_err(|error| CliError::Message(error.to_string()))
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Missing(group)) => {
            eprintln!("preproc: Rust port is not implemented yet: {group}");
            ExitCode::from(86)
        }
        Err(CliError::Internal(message)) => {
            eprintln!("internal compiler error: {message}");
            ExitCode::from(70)
        }
        Err(CliError::Message(message)) => {
            eprintln!("ERROR: {message}");
            ExitCode::FAILURE
        }
        Err(CliError::Diagnostic(diagnostic)) => {
            let mut stderr = io::stderr().lock();
            let _status = stderr.write_all(b"ERROR: ");
            let _status = stderr.write_all(&diagnostic);
            let _status = stderr.write_all(b"\n");
            ExitCode::FAILURE
        }
    }
}
