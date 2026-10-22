// Translated from cppgm++.cpp's AST invocation and source loop. See NOTICE.
use cppgm_rust::{
    driver::ast::{AstToolError, Destination, EmitMode, LowIrOutput, emit_unit},
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
#[path = "source_tool/build_time.rs"]
mod build_time;
#[path = "source_tool/stack_limit.rs"]
mod stack_limit;
enum CliError {
    Message(String),
    Diagnostic(Vec<u8>),
    Missing(&'static str),
    Internal(&'static str),
    Output,
}
// Like the oracle ofstream, failed output does not stop parsing later inputs.
struct Output {
    file: Option<io::BufWriter<File>>,
    failed: bool,
}
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .file
            .as_mut()
            .is_some_and(|file| file.write_all(bytes).is_err())
        {
            self.file = None;
            self.failed = true;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.file.as_mut().is_some_and(|file| file.flush().is_err()) {
            self.file = None;
            self.failed = true;
        }
        Ok(())
    }
}
fn path_error(path: &std::ffi::OsStr, message: &[u8]) -> CliError {
    let mut bytes = path.as_bytes().to_vec();
    bytes.extend_from_slice(b": ");
    bytes.extend_from_slice(message);
    CliError::Diagnostic(bytes)
}
fn invocation(args: &[OsString]) -> Result<(&std::ffi::OsStr, Vec<&std::ffi::OsStr>), CliError> {
    if !args.iter().any(|arg| {
        arg == "--emit-ast"
            || arg == "--emit-types"
            || arg == "--emit-semantics"
            || arg == "--emit-lowir"
    }) {
        return Err(CliError::Missing("source compiler modes beyond --emit-ast"));
    }
    let lowir = args.iter().any(|arg| arg == "--emit-lowir");
    let mut output = None;
    let mut inputs = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        index += 1;
        if arg == "--emit-ast"
            || arg == "--emit-types"
            || arg == "--emit-semantics"
            || arg == "--emit-lowir"
            || lowir && arg == "-O0"
        {
            continue;
        }
        if arg == "-o" {
            let value = args
                .get(index)
                .ok_or_else(|| CliError::Message("missing output file after -o".to_owned()))?;
            output = Some(value.as_os_str());
            index += 1;
            continue;
        }
        if matches!(arg.as_bytes(), b"--stats" | b"--hosted")
            || arg.as_bytes().starts_with(b"-I")
            || arg.as_bytes().starts_with(b"-D")
            || arg.as_bytes().starts_with(b"-U")
        {
            return Err(CliError::Missing(
                "AST statistics and hosted/preprocessing invocation options",
            ));
        }
        if arg.as_bytes().starts_with(b"-") {
            return Err(CliError::Message(format!(
                "unsupported option in emit mode: {}",
                arg.to_string_lossy()
            )));
        }
        inputs.push(arg.as_os_str());
    }
    let output = output.ok_or_else(|| CliError::Message("invalid usage".to_owned()))?;
    if inputs.is_empty() {
        return Err(CliError::Message("invalid usage".to_owned()));
    }
    Ok((output, inputs))
}
fn run() -> Result<(), CliError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !args.iter().any(|arg| {
        matches!(
            arg.as_bytes(),
            b"--emit-ast" | b"--emit-types" | b"--emit-semantics" | b"--emit-lowir"
        )
    }) {
        use std::os::unix::fs::PermissionsExt;
        let values = build_time::query().map_err(CliError::Message)?;
        let budget = cppgm_rust::support::stack::StackBudget::from_limit(stack_limit::query())
            .map_err(CliError::Internal)?;
        return cppgm_rust::driver::toolchain::run(
            &args,
            &values,
            budget,
            &std::fs::Permissions::from_mode(0o755),
        )
        .map_err(tool_error);
    }
    let ast = args.iter().any(|arg| arg == "--emit-ast");
    let types = args.iter().any(|arg| arg == "--emit-types");
    let semantics = args.iter().any(|arg| arg == "--emit-semantics");
    let lowir = args.iter().any(|arg| arg == "--emit-lowir");
    if u8::from(lowir) + u8::from(ast) + u8::from(types) + u8::from(semantics) > 1 {
        return Err(CliError::Message(
            "multiple --emit-* options provided".to_owned(),
        ));
    }
    let mode = if lowir {
        EmitMode::LowIr
    } else if semantics {
        EmitMode::Semantics
    } else if types {
        EmitMode::Types
    } else {
        EmitMode::Ast
    };
    let (destination, inputs) = invocation(&args)?;
    let values = build_time::query().map_err(CliError::Message)?;
    let stack_budget = cppgm_rust::support::stack::StackBudget::from_limit(stack_limit::query())
        .map_err(CliError::Internal)?;
    let file = if lowir {
        None
    } else {
        File::create(destination).ok().map(io::BufWriter::new)
    };
    let failed = !lowir && file.is_none();
    let mut output = Output { file, failed };
    let mut lowir_output = LowIrOutput::default();
    if !lowir {
        writeln!(output, "{} translation units", inputs.len()).map_err(|_| CliError::Output)?;
    }
    for (index, path) in inputs.iter().enumerate() {
        if !std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
            return Err(path_error(path, b"cannot open the source file"));
        }
        let mut file =
            File::open(path).map_err(|_| path_error(path, b"cannot open the source file"))?;
        let mut bytes = Vec::new();
        let _read_status = file.read_to_end(&mut bytes);
        let source =
            SourceBuffer::new(bytes).map_err(|message| CliError::Message(message.to_owned()))?;
        emit_unit(
            &source,
            path.as_bytes(),
            &values,
            if lowir {
                Destination::LowIr(&mut lowir_output)
            } else {
                Destination::Text(&mut output)
            },
            index + 1,
            stack_budget,
            mode,
        )
        .map_err(|error| match error {
            AstToolError::Missing(group) => CliError::Missing(group),
            AstToolError::Internal(message) => CliError::Internal(message),
            AstToolError::Diagnostic(bytes) | AstToolError::NativeDiagnostic(bytes) => {
                CliError::Diagnostic(bytes)
            }
            AstToolError::Output(_) => CliError::Output,
            AstToolError::Initialization(PreprocessError::NotImplemented(group)) => {
                CliError::Missing(group)
            }
            AstToolError::Initialization(PreprocessError::Invariant(message)) => {
                CliError::Internal(message)
            }
            AstToolError::Initialization(error) => {
                let mut bytes = path.as_bytes().to_vec();
                bytes.push(b':');
                bytes.extend_from_slice(
                    cppgm_rust::driver::preproc::describe(&source, &error).as_bytes(),
                );
                CliError::Diagnostic(bytes)
            }
        })?;
    }
    if lowir {
        output.file = File::create(destination).ok().map(io::BufWriter::new);
        output.failed = output.file.is_none();
        lowir_output
            .write(&mut output)
            .map_err(|_| CliError::Output)?;
    }
    output.flush().map_err(|_| CliError::Output)?;
    if output.failed {
        if lowir {
            return Err(CliError::Diagnostic(
                [b"cannot write ".as_slice(), destination.as_bytes()].concat(),
            ));
        }
        return Err(CliError::Output);
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Output) => ExitCode::FAILURE,
        Err(CliError::Missing(group)) => {
            eprintln!("cppgm++: Rust port is not implemented yet: {group}");
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
        Err(CliError::Diagnostic(bytes)) => {
            let mut stderr = io::stderr().lock();
            let _status = stderr.write_all(b"ERROR: ");
            let _status = stderr.write_all(&bytes);
            let _status = stderr.write_all(b"\n");
            ExitCode::FAILURE
        }
    }
}

fn tool_error(error: AstToolError) -> CliError {
    match error {
        AstToolError::Missing(group)
        | AstToolError::Initialization(PreprocessError::NotImplemented(group)) => {
            CliError::Missing(group)
        }
        AstToolError::Internal(message)
        | AstToolError::Initialization(PreprocessError::Invariant(message)) => {
            CliError::Internal(message)
        }
        AstToolError::Diagnostic(bytes) | AstToolError::NativeDiagnostic(bytes) => {
            CliError::Diagnostic(bytes)
        }
        AstToolError::Output(_) => CliError::Output,
        AstToolError::Initialization(_) => {
            CliError::Internal("unrendered source initialization error")
        }
    }
}
