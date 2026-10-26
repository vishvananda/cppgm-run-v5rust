// Translated from dev/preproc.cpp. See NOTICE.
use cppgm_rust::{
    driver::preproc::{PreprocToolError, describe, emit_preprocessed},
    preprocess::error::PreprocessError,
    support::source::SourceBuffer,
};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Read, Write},
    os::unix::ffi::OsStrExt,
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
    Statistics {
        error: Box<Self>,
        path: Vec<u8>,
        counters: Box<cppgm_rust::preprocess::counters::PreprocessorCounters>,
    },
}
use cppgm_rust::driver::batch as batch_status;
#[path = "source_tool/build_time.rs"]
mod build_time;

fn run() -> Result<(), CliError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    run_with_args(&args)
}
fn path_diagnostic(path: &std::ffi::OsStr, message: &[u8]) -> CliError {
    let mut diagnostic = path.as_bytes().to_vec();
    diagnostic.extend_from_slice(b": ");
    diagnostic.extend_from_slice(message);
    CliError::Diagnostic(diagnostic)
}
fn run_with_args(args: &[OsString]) -> Result<(), CliError> {
    if args.iter().any(|arg| arg == "--batch-stdin") {
        let mut output = io::stdout().lock();
        batch_status::write(&mut io::stdin().lock(), &mut output);
        return Ok(());
    }
    let stats = args.first().is_some_and(|arg| arg == "--stats");
    let args = if stats { &args[1..] } else { args };
    if args.len() < 3 || args[0] != "-o" {
        return Err(CliError::Message("invalid usage".to_owned()));
    }
    let values = build_time::query().map_err(CliError::Message)?;
    let mut output = OutputStream {
        file: File::create(&args[1]).ok().map(io::BufWriter::new),
    };
    writeln!(output, "preproc {}", args.len() - 2)
        .map_err(|error| CliError::Message(error.to_string()))?;
    for path in &args[2..] {
        output
            .write_all(b"sof ")
            .and_then(|()| output.write_all(path.as_bytes()))
            .and_then(|()| output.write_all(b"\n"))
            .map_err(|error| CliError::Message(error.to_string()))?;
        let mut bytes = Vec::new();
        // Match SourceFiles' stat-before-open boundary, including nonregular
        // inputs that could block if opened (such as a FIFO).
        if !std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
            return Err(path_diagnostic(path, b"cannot open the source file"));
        }
        let mut file =
            File::open(path).map_err(|_| path_diagnostic(path, b"cannot open the source file"))?;
        let _read_status = file.read_to_end(&mut bytes);
        let source =
            SourceBuffer::new(bytes).map_err(|message| CliError::Message(message.to_owned()))?;
        let mut counters = None;
        let result = emit_preprocessed(
            &source,
            path.as_bytes(),
            &values,
            &mut output,
            &mut counters,
        )
        .map_err(|error| match error {
            PreprocToolError::Initialization(PreprocessError::Source(error)) => {
                CliError::Message(error.to_string())
            }
            PreprocToolError::Initialization(error) => match error {
                PreprocessError::NotImplemented(group) => CliError::Missing(group),
                PreprocessError::Invariant(message) => CliError::Internal(message),
                _ => {
                    let mut diagnostic = path.as_bytes().to_vec();
                    diagnostic.push(b':');
                    diagnostic.extend_from_slice(describe(&source, &error).as_bytes());
                    CliError::Diagnostic(diagnostic)
                }
            },
            PreprocToolError::Preprocess { error, diagnostic } => match error {
                PreprocessError::NotImplemented(group) => CliError::Missing(group),
                PreprocessError::Invariant(message) => CliError::Internal(message),
                _ => CliError::Diagnostic(diagnostic),
            },
            PreprocToolError::InvalidToken => path_diagnostic(path, b"invalid phase-7 token"),
            PreprocToolError::Output(error) => CliError::Message(error.to_string()),
        });
        if stats && let Some(counters) = counters {
            if let Err(error) = result {
                return Err(CliError::Statistics {
                    error: Box::new(error),
                    path: path.as_bytes().to_vec(),
                    counters: Box::new(counters),
                });
            }
            report_counters(path.as_bytes(), &counters);
        } else {
            result?;
        }
    }
    output
        .flush()
        .map_err(|error| CliError::Message(error.to_string()))
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report_error(error),
    }
}
fn report_error(error: CliError) -> ExitCode {
    match error {
        CliError::Missing(group) => {
            let _ = writeln!(
                io::stderr().lock(),
                "preproc: Rust port is not implemented yet: {group}"
            );
            ExitCode::from(86)
        }
        CliError::Internal(message) => {
            let _ = writeln!(io::stderr().lock(), "internal compiler error: {message}");
            ExitCode::from(70)
        }
        CliError::Message(message) => {
            let _ = writeln!(io::stderr().lock(), "ERROR: {message}");
            ExitCode::FAILURE
        }
        CliError::Diagnostic(diagnostic) => {
            let mut stderr = io::stderr().lock();
            let _status = stderr.write_all(b"ERROR: ");
            let _status = stderr.write_all(&diagnostic);
            let _status = stderr.write_all(b"\n");
            ExitCode::FAILURE
        }
        CliError::Statistics {
            error,
            path,
            counters,
        } => {
            let status = report_error(*error);
            report_counters(&path, &counters);
            status
        }
    }
}
fn report_counters(path: &[u8], counters: &cppgm_rust::preprocess::counters::PreprocessorCounters) {
    let mut stderr = io::stderr().lock();
    let _status = stderr.write_all(b"preproc stats ");
    let _status = stderr.write_all(path);
    let expansion = &counters.expansion;
    let _status = writeln!(
        stderr,
        ": source_tokens={} directives={} skipped_directives={} includes={} guarded_skips={} files={} source_bytes={} identifiers={} arena_bytes={} invocations={} produced_tokens={} argument_tokens={} argument_expansions={} nested_collections={} pastes={} stringizes={} raw_operands_hidden={} peak_contexts={} peak_context_tokens={} peak_invocations={}",
        counters.source_tokens,
        counters.directives,
        counters.skipped_directives,
        counters.includes,
        counters.guarded_skips,
        counters.files,
        counters.source_bytes,
        counters.identifiers,
        counters.arena_bytes,
        expansion.invocations,
        expansion.produced_tokens,
        expansion.argument_tokens,
        expansion.argument_expansions,
        expansion.nested_collections,
        expansion.pastes,
        expansion.stringizes,
        expansion.raw_operands_hidden,
        expansion.peak_contexts,
        expansion.peak_context_tokens,
        expansion.peak_invocations
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn no_runner_batch_reads_request_bytes_including_an_unterminated_last_line() {
        let mut output = Vec::new();
        batch_status::write(&mut &b"\xff\n\nlast\0"[..], &mut output);
        assert_eq!(
            output,
            b"EXIT_NOT_IMPLEMENTED\nEXIT_NOT_IMPLEMENTED\nEXIT_NOT_IMPLEMENTED\n"
        );
        output.clear();
        batch_status::write(&mut &b""[..], &mut output);
        assert!(output.is_empty());
    }

    #[test]
    fn file_boundary_preserves_path_bytes_and_rejects_directories() {
        let root = std::env::temp_dir().join(format!("pa4-cli-audit-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let input = root.join(OsString::from_vec(b"source-\xff.cpp".to_vec()));
        let output = root.join("tokens");
        let mut args = vec![
            OsString::from("-o"),
            output.into_os_string(),
            input.clone().into_os_string(),
        ];
        std::fs::write(&input, b"17\n").unwrap();
        assert!(run_with_args(&args).is_ok());
        let mut expected = b"preproc 1\nsof ".to_vec();
        expected.extend_from_slice(input.as_os_str().as_bytes());
        expected.extend_from_slice(b"\nliteral 17 int 11000000\neof\n");
        assert_eq!(std::fs::read(&args[1]).unwrap(), expected);
        std::fs::write(&input, b"#error\n").unwrap();
        let Err(CliError::Diagnostic(diagnostic)) = run_with_args(&args) else {
            panic!("expected a located preprocessing rejection");
        };
        let mut expected = input.as_os_str().as_bytes().to_vec();
        expected.extend_from_slice(b":1:2: #error");
        assert_eq!(diagnostic, expected);
        args[2] = root.clone().into_os_string();
        let Err(CliError::Diagnostic(diagnostic)) = run_with_args(&args) else {
            panic!("expected a source-open rejection for a directory");
        };
        let mut expected = root.as_os_str().as_bytes().to_vec();
        expected.extend_from_slice(b": cannot open the source file");
        assert_eq!(diagnostic, expected);
        std::fs::remove_dir_all(root).unwrap();
    }
}
