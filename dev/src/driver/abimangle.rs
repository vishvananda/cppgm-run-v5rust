//! abimangle invocation, byte diagnostics and output-file lifetime. See NOTICE.
use crate::abi::{Error, facts};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, BufWriter, Write},
    path::PathBuf,
};

#[must_use]
pub fn run(
    arguments: &[OsString],
    budget: crate::support::stack::StackBudget,
) -> std::process::ExitCode {
    match execute(arguments, budget) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(Error::NotImplemented(feature)) => {
            eprintln!("abimangle: Rust port is not implemented yet: {feature}");
            std::process::ExitCode::from(86)
        }
        Err(error) => {
            let message = match error {
                Error::Diagnostic(message) => message,
                _ => b"ABI storage limit exceeded".to_vec(),
            };
            let mut stderr = io::stderr().lock();
            let _ = stderr
                .write_all(b"abimangle: ")
                .and_then(|()| stderr.write_all(&message))
                .and_then(|()| stderr.write_all(b"\n"));
            std::process::ExitCode::FAILURE
        }
    }
}
fn execute(
    arguments: &[OsString],
    budget: crate::support::stack::StackBudget,
) -> Result<(), Error> {
    if arguments.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("usage: abimangle -o <outfile> <abi-facts-file>...\n\nquery flags:\n  --help, -h\n");
        return Ok(());
    }
    let mut output = OsStr::new("");
    let mut paths = Vec::new();
    let mut stats = false;
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        if argument == "-o" {
            let next = arguments
                .next()
                .ok_or_else(|| Error::diagnostic("missing output file after -o"))?;
            if !output.is_empty() {
                return Err(Error::diagnostic("repeated option: -o"));
            }
            output = next;
        } else if argument == "--stats" {
            stats = true;
        } else if argument.as_encoded_bytes().len() > 1 && argument.as_encoded_bytes()[0] == b'-' {
            return Err(Error::word(
                "unknown option: ",
                argument.as_encoded_bytes(),
                "",
            ));
        } else {
            paths.push(PathBuf::from(argument));
        }
    }
    if output.is_empty() || paths.is_empty() {
        return Err(Error::diagnostic("invalid usage"));
    }
    if stats {
        return Err(Error::NotImplemented("ABI statistics"));
    }
    let file = File::create(output).map_err(|_| {
        Error::word(
            "unable to open output file '",
            output.as_encoded_bytes(),
            "'",
        )
    })?;
    // The frozen fopen output uses a 4-KiB stdio buffer on this target. Keep
    // small writes deferred until close and large writes at the reader flush
    // boundary, including the distinct diagnostics at those two boundaries.
    let mut file = BufWriter::with_capacity(4096, file);
    facts::mangle_files(&paths, &mut file, budget)?;
    file.flush().map_err(|_| {
        Error::word(
            "unable to write output file '",
            output.as_encoded_bytes(),
            "'",
        )
    })
}
