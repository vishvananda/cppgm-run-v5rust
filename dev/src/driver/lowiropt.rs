//! Standalone optimizer boundary, translated from lowiropt.cpp. See NOTICE.
use crate::{lowir, opt};
use lowir::Error;
use std::{
    ffi::OsString,
    io::{self, BufRead, Write},
    path::PathBuf,
};

const HELP: &str = "usage: lowiropt -O0 -o <outfile> <lowirfile>...
   or: lowiropt -O1 -o <outfile> <lowirfile>...
   or: lowiropt -O2 -o <outfile> <lowirfile>...
   or: lowiropt -O3 -o <outfile> <lowirfile>...

query flags:
  --help, -h

required flags:
  -O0 / -O1 / -O2 / -O3
  -o <outfile>
";

#[derive(Default)]
struct Invocation {
    level: Option<u8>,
    output: PathBuf,
    inputs: Vec<PathBuf>,
    limits: opt::Limits,
    validate: bool,
    stats: bool,
    verify: bool,
}

/// Run the optimizer with arguments collected by its executable.
#[must_use]
pub fn run(args: &[OsString]) -> std::process::ExitCode {
    match execute(args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(Error::NotImplemented(feature)) => {
            eprintln!("lowiropt: Rust port is not implemented yet: {feature}");
            std::process::ExitCode::from(86)
        }
        Err(Error::Construction(message)) => {
            eprintln!("internal compiler error: {message}");
            std::process::ExitCode::from(70)
        }
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = stderr.write_all(b"ERROR: ").and_then(|()| match error {
                Error::Diagnostic(message) => stderr.write_all(message.as_bytes()),
                Error::ByteDiagnostic(bytes) => stderr.write_all(&bytes),
                Error::Output(error) => write!(stderr, "{error}"),
                Error::Limit => stderr.write_all(b"LowIR storage limit exceeded"),
                Error::Construction(_) | Error::NotImplemented(_) => Ok(()),
            });
            let _ = stderr.write_all(b"\n");
            std::process::ExitCode::FAILURE
        }
    }
}

fn execute(args: &[OsString]) -> Result<(), Error> {
    if args.iter().any(|arg| arg == "--batch-stdin") {
        let mut out = io::stdout().lock();
        for line in io::stdin().lock().split(b'\n') {
            let _ = line.map_err(Error::Output)?;
            writeln!(out, "EXIT_NOT_IMPLEMENTED").map_err(Error::Output)?;
            out.flush().map_err(Error::Output)?;
        }
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return io::stdout()
            .lock()
            .write_all(HELP.as_bytes())
            .map_err(Error::Output);
    }
    let invocation = parse(args)?;
    let (mut program, _) = lowir::read_files(&invocation.inputs, false)?;
    if invocation.stats || invocation.verify {
        return Err(Error::NotImplemented(
            "LowIR optimizer telemetry and pass verification",
        ));
    }
    opt::optimize_with_limits(
        &mut program,
        invocation.level.ok_or(Error::Limit)?,
        invocation.limits,
        false,
    )?;
    if invocation.validate {
        lowir::validator::validate_constructed(&program).map_err(|error| {
            let message = match error {
                Error::Diagnostic(message) => message,
                Error::ByteDiagnostic(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                error => return error,
            };
            Error::Construction(format!("optimized LowIR is invalid: {message}"))
        })?;
    }
    lowir::write_file(&program, &invocation.output)
}

fn parse(args: &[OsString]) -> Result<Invocation, Error> {
    let mut invocation = Invocation::default();
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_encoded_bytes() {
            [b'-', b'O', level @ b'0'..=b'3'] => {
                if invocation.level.replace(level - b'0').is_some() {
                    return Err(diagnostic("multiple optimization levels provided"));
                }
            }
            b"-o" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| diagnostic("missing output file after -o"))?;
                if !invocation.output.as_os_str().is_empty() {
                    return Err(diagnostic("multiple output files provided"));
                }
                invocation.output = value.into();
            }
            b"--inline-limit" => {
                let valid = arguments.next().is_some_and(|value| {
                    invocation.limits.parse_override(value.as_encoded_bytes())
                });
                if !valid {
                    return Err(diagnostic("invalid --inline-limit"));
                }
            }
            b"--stats" => invocation.stats = true,
            b"--validate" => invocation.validate = true,
            b"--verify-passes" => invocation.verify = true,
            bytes if bytes.starts_with(b"--verify-passes=") => invocation.verify = true,
            bytes if bytes.starts_with(b"-") => {
                return Err(Error::argument("unknown option: ", argument));
            }
            _ => invocation.inputs.push(argument.into()),
        }
    }
    if invocation.level.is_none()
        || invocation.output.as_os_str().is_empty()
        || invocation.inputs.is_empty()
    {
        return Err(diagnostic(
            "usage: lowiropt -O<0-3> -o <outfile> <lowirfile>...",
        ));
    }
    Ok(invocation)
}

fn diagnostic(message: &str) -> Error {
    Error::Diagnostic(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    #[test]
    fn inline_limits_keep_the_full_target_range_and_last_value() {
        let mut limits = opt::Limits::default();
        assert!(limits.parse_override(b"once-cap=4294967295"));
        assert_eq!(limits.once_cap, u32::MAX);
        assert!(limits.parse_override(b"once-cap=00012"));
        assert_eq!(limits.once_cap, 12);
        for bytes in [
            b"once-cap=0".as_slice(),
            b"once-cap=4294967296",
            b"once-cap=-1",
            b"once-cap=",
            b"once-cap=1=2",
            b"unknown=1",
        ] {
            assert!(!limits.parse_override(bytes));
            assert_eq!(limits.once_cap, 12);
        }
    }

    #[test]
    fn option_presence_and_diagnostics_preserve_oracle_order_and_bytes() {
        let args = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
        for (values, message) in [
            (vec!["-O1", "-O0"], "multiple optimization levels provided"),
            (vec!["-o"], "missing output file after -o"),
            (
                vec!["-o", "one", "-o", "two"],
                "multiple output files provided",
            ),
            (vec!["--inline-limit", "bad"], "invalid --inline-limit"),
        ] {
            assert!(
                matches!(parse(&args(&values)), Err(Error::Diagnostic(text)) if text == message)
            );
        }
        let invocation = parse(&args(&["-O0", "-o", "", "-o", "out", "input"])).unwrap();
        assert_eq!(invocation.output, PathBuf::from("out"));
        let invalid = OsString::from_vec(b"-\xff".to_vec());
        assert!(
            matches!(parse(&[invalid]), Err(Error::ByteDiagnostic(bytes)) if bytes == b"unknown option: -\xff")
        );
    }
}
