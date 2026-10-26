//! PA8 command-line routing; shared phases retain all compiler work. See NOTICE.
use crate::lowir::{self, Error};
use std::{
    ffi::{OsStr, OsString},
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

/// Run `LowIR` with configuration collected at the executable boundary.
#[must_use]
pub fn run(args: &[OsString], peak_rss: fn() -> u64) -> std::process::ExitCode {
    match execute(args, peak_rss) {
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
        Err(Error::PassVerification(message)) => {
            let _ = io::stderr().lock().write_all(&message);
            std::process::ExitCode::from(3)
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
fn execute(args: &[OsString], peak_rss: fn() -> u64) -> Result<(), Error> {
    if args.iter().any(|a| a == "--batch-stdin") {
        let mut out = io::stdout().lock();
        for line in io::stdin().lock().split(b'\n') {
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
    let mut stats = false;
    let mut output = OsStr::new("");
    let mut exercise = OsStr::new("");
    let mut paths = Vec::new();
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        match argument.to_str().unwrap_or("") {
            option @ ("-o" | "--exercise") => {
                let value = arguments
                    .next()
                    .ok_or_else(|| Error::Diagnostic(format!("missing value after {option}")))?;
                let destination = if option == "-o" {
                    &mut output
                } else {
                    &mut exercise
                };
                if !destination.is_empty() {
                    return Err(Error::Diagnostic(format!("repeated option: {option}")));
                }
                *destination = value;
            }
            "--stats" => stats = true,
            _ if argument.as_encoded_bytes().starts_with(b"-") => {
                return Err(Error::argument("unknown option: ", argument));
            }
            _ => paths.push(PathBuf::from(argument)),
        }
    }
    if output.is_empty() || paths.is_empty() == exercise.is_empty() {
        return Err(Error::Diagnostic(
            "expected input files or one exercise".into(),
        ));
    }
    let report = if exercise.is_empty() {
        lowir::roundtrip(&paths, Path::new(output), stats)?
    } else {
        let name = exercise
            .to_str()
            .filter(|name| matches!(*name, "sum" | "swap" | "call"))
            .ok_or_else(|| Error::argument("unknown LowIR exercise: ", exercise))?;
        lowir::exercise(name, Path::new(output), stats)?
    };
    if stats {
        report
            .write(&mut io::stderr().lock(), peak_rss())
            .map_err(Error::Output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn non_utf8_options_and_exercises_report_original_bytes() {
        for (args, expected) in [
            (
                vec![OsString::from_vec(b"-\xff".to_vec())],
                b"unknown option: -\xff".as_slice(),
            ),
            (
                vec![
                    OsString::from("--exercise"),
                    OsString::from_vec(vec![255]),
                    OsString::from("-o"),
                    OsString::from("unused"),
                ],
                b"unknown LowIR exercise: \xff".as_slice(),
            ),
        ] {
            let result = execute(&args, || 0);
            assert!(matches!(result, Err(Error::ByteDiagnostic(message)) if message == expected));
        }
    }
}
