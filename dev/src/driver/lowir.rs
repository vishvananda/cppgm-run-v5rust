//! PA8 command-line routing; shared phases retain all compiler work. See NOTICE.
use crate::lowir::{self, Error};
use std::io::{self, BufRead, Write};

/// Run `LowIR` with configuration collected at the executable boundary.
#[must_use]
pub fn run(args: &[String]) -> std::process::ExitCode {
    match execute(args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(Error::Diagnostic(message)) => {
            eprintln!("ERROR: {message}");
            std::process::ExitCode::FAILURE
        }
        Err(Error::ByteDiagnostic(message)) => {
            let mut stderr = io::stderr().lock();
            let _ = stderr
                .write_all(b"ERROR: ")
                .and_then(|()| stderr.write_all(&message))
                .and_then(|()| stderr.write_all(b"\n"));
            std::process::ExitCode::FAILURE
        }
        Err(Error::Construction(message)) => {
            eprintln!("internal compiler error: invalid exercise program: {message}");
            std::process::ExitCode::from(70)
        }
        Err(Error::NotImplemented(feature)) => {
            eprintln!("lowir: Rust port is not implemented yet: {feature}");
            std::process::ExitCode::from(86)
        }
        Err(Error::Limit) => {
            eprintln!("ERROR: LowIR storage limit exceeded");
            std::process::ExitCode::FAILURE
        }
        Err(Error::Output(error)) => {
            eprintln!("ERROR: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn execute(args: &[String]) -> Result<(), Error> {
    if args.iter().any(|a| a == "--batch-stdin") {
        let mut out = io::stdout().lock();
        for line in io::stdin().lock().lines() {
            let _ = line.map_err(Error::Output)?;
            writeln!(out, "EXIT_NOT_IMPLEMENTED").map_err(Error::Output)?;
            out.flush().map_err(Error::Output)?;
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!(
            "usage: lowir -o <outfile> <lowirfile>...\n   or: lowir --exercise <sum|swap|call> -o <outfile>\n\nquery flags:\n  --help, -h\n\nrequired flags:\n  -o <outfile>\n"
        );
        return Ok(());
    }
    let mut output = "";
    let mut exercise = "";
    let mut paths = Vec::new();
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-o" | "--exercise" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| Error::Diagnostic(format!("missing value after {argument}")))?;
                let destination = if argument == "-o" {
                    &mut output
                } else {
                    &mut exercise
                };
                if !destination.is_empty() {
                    return Err(Error::Diagnostic(format!("repeated option: {argument}")));
                }
                *destination = value;
            }
            "--stats" => return Err(Error::NotImplemented("LowIR model statistics")),
            a if a.starts_with('-') => {
                return Err(Error::Diagnostic(format!("unknown option: {a}")));
            }
            _ => paths.push(argument.clone()),
        }
    }
    if output.is_empty() || paths.is_empty() == exercise.is_empty() {
        return Err(Error::Diagnostic(
            "expected input files or one exercise".into(),
        ));
    }
    if !exercise.is_empty() {
        if !matches!(exercise, "sum" | "swap" | "call") {
            return Err(Error::Diagnostic(format!(
                "unknown LowIR exercise: {exercise}"
            )));
        }
        return lowir::exercise(exercise, output);
    }
    lowir::roundtrip(&paths, output)
}
