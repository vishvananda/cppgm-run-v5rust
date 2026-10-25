//! Direct PA29 queries use immutable build-selected configuration bytes.
use crate::CliError;
use crate::builtin_host_configuration as captured;
use std::{ffi::OsString, io::Write, os::unix::ffi::OsStrExt};

pub(super) fn run(args: &[OsString]) -> Result<bool, CliError> {
    let Some(first) = args.first() else {
        return Ok(false);
    };
    let bytes = match first.as_bytes() {
        b"--version" | b"-v" => [
            b"cppgm++ (host configuration: ".as_slice(),
            captured::COMMAND,
            b")\n",
        ]
        .concat(),
        b"-dumpmachine" => captured::TARGET.to_vec(),
        b"-dumpversion" => captured::VERSION.to_vec(),
        b"-print-search-dirs" => captured::SEARCH.to_vec(),
        _ => return Ok(false),
    };
    if args.len() != 1 {
        return Err(CliError::Message(
            "query flag must be used as a direct invocation".into(),
        ));
    }
    let mut output = std::io::stdout().lock();
    output
        .write_all(&bytes)
        .and_then(|()| output.flush())
        .map_err(|_| CliError::Output)?;
    Ok(true)
}
