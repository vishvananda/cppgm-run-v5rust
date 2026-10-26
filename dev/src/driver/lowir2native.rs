//! Standalone native-tool invocation and diagnostic boundary. See NOTICE.
use crate::{lowir::Error, native};
use std::{
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::PathBuf,
};

const HELP: &str = "usage: lowir2native -o <outfile> <lowirfile>...
   or: lowir2native -O0 -o <outfile> <lowirfile>...
   or: lowir2native -O1 -o <outfile> <lowirfile>...
   or: lowir2native -O2 -o <outfile> <lowirfile>...
   or: lowir2native -O3 -o <outfile> <lowirfile>...
   or: lowir2native --dump-machine-ir <mirfile> <lowirfile>...
   or: lowir2native -o <outfile> --dump-machine-ir <mirfile> <lowirfile>...

query flags:
  --help, -h

driver flags:
  -O0 / -O1 / -O2 / -O3
  -o <outfile>
  --target <target>
  --stats
  --dump-machine-ir <mirfile>
";

#[derive(Default)]
struct Invocation {
    options: native::Options,
    sources: Vec<PathBuf>,
}

/// Run the standalone backend with byte-preserving arguments.
#[must_use]
pub fn run(
    args: &[OsString],
    peak_rss: fn() -> u64,
    permissions: std::fs::Permissions,
) -> std::process::ExitCode {
    match execute(args, peak_rss, permissions) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(Error::NotImplemented(feature)) => {
            let _status = writeln!(
                io::stderr().lock(),
                "lowir2native: Rust port is not implemented yet: {feature}"
            );
            std::process::ExitCode::from(86)
        }
        Err(Error::Construction(message)) => {
            let _status = writeln!(io::stderr().lock(), "internal compiler error: {message}");
            std::process::ExitCode::from(70)
        }
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = write_error(&mut stderr, error);
            std::process::ExitCode::FAILURE
        }
    }
}

fn write_error(out: &mut impl Write, error: Error) -> io::Result<()> {
    out.write_all(b"ERROR: ")?;
    match error {
        Error::ByteDiagnostic(bytes) => out.write_all(&bytes)?,
        Error::Diagnostic(message) => out.write_all(message.as_bytes())?,
        Error::Output(error) => write!(out, "{error}")?,
        Error::Limit => out.write_all(b"LowIR storage limit exceeded")?,
        Error::Construction(_) | Error::NotImplemented(_) | Error::PassVerification(_) => {
            return Ok(());
        }
    }
    out.write_all(b"\n")
}

fn execute(
    args: &[OsString],
    peak_rss: fn() -> u64,
    permissions: std::fs::Permissions,
) -> Result<(), Error> {
    if args.iter().any(|arg| arg == "--batch-stdin") {
        super::batch::write(&mut io::stdin().lock(), &mut io::stdout().lock());
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        let _ = io::stdout().lock().write_all(HELP.as_bytes());
        return Ok(());
    }
    let mut invocation = parse(args)?;
    invocation.options.permissions = Some(permissions);
    let (mut program, _) = crate::lowir::read_files(&invocation.sources, false)?;
    let report = native::compile(&mut program, &invocation.options)?;
    if invocation.options.stats {
        let _status = report.write(&mut io::stderr().lock(), peak_rss());
    }
    Ok(())
}

fn parse(args: &[OsString]) -> Result<Invocation, Error> {
    let mut invocation = Invocation::default();
    let mut target = OsStr::new("");
    let mut level = None;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        let bytes = argument.as_encoded_bytes();
        if matches!(bytes, [b'-', b'O', b'0'..=b'3']) {
            if level.is_some() {
                return Err(diagnostic("multiple optimization levels provided"));
            }
            level = Some(bytes[2] - b'0');
            continue;
        }
        match argument.to_str().unwrap_or("") {
            "--stats" => invocation.options.stats = true,
            "--target" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| diagnostic("missing target after --target"))?;
                if !target.is_empty() {
                    return Err(diagnostic("multiple --target options provided"));
                }
                target = value;
                if target != "linux" {
                    return Err(Error::argument("unsupported target: ", target));
                }
            }
            option @ ("-o" | "--dump-machine-ir" | "--dump-native-plan") => {
                let output = option == "-o";
                let value = arguments.next().ok_or_else(|| {
                    diagnostic(if output {
                        "missing output file after -o"
                    } else {
                        "missing output file after --dump-machine-ir"
                    })
                })?;
                let path = if output {
                    &mut invocation.options.output
                } else {
                    &mut invocation.options.machine_ir
                };
                if !path.as_os_str().is_empty() {
                    return Err(diagnostic(if output {
                        "multiple output files provided"
                    } else {
                        "multiple machine IR dump paths provided"
                    }));
                }
                *path = value.into();
            }
            _ if bytes.starts_with(b"-") => {
                return Err(Error::argument("unknown option: ", argument));
            }
            _ => invocation.sources.push(argument.into()),
        }
    }
    if (invocation.options.output.as_os_str().is_empty()
        && invocation.options.machine_ir.as_os_str().is_empty())
        || invocation.sources.is_empty()
    {
        return Err(diagnostic("invalid usage"));
    }
    invocation.options.optimization_level = level.unwrap_or(0);
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
    fn option_presence_follows_oracle_empty_path_rules() -> Result<(), Error> {
        let args: Vec<OsString> = ["-o", "", "-o", "output", "-O0", "source"]
            .into_iter()
            .map(OsString::from)
            .collect();
        let invocation = parse(&args)?;
        assert_eq!(invocation.options.output, PathBuf::from("output"));
        let repeated: Vec<OsString> = ["-O0", "-O0"].into_iter().map(OsString::from).collect();
        assert!(matches!(parse(&repeated), Err(Error::Diagnostic(text))
            if text == "multiple optimization levels provided"));
        Ok(())
    }

    #[test]
    fn diagnostics_retain_non_unicode_argument_bytes() {
        let argument = OsString::from_vec(b"-\xff".to_vec());
        assert!(
            matches!(parse(&[argument]), Err(Error::ByteDiagnostic(text))
            if text == b"unknown option: -\xff")
        );
    }
}
