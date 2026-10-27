// Translated from cppgm++.cpp's AST invocation and source loop. See NOTICE.
use cppgm_rust::{
    driver::ast::{
        AstToolError, Destination, EmitMode, LowIrOutput, UnitOptions, emit_configured_unit,
    },
    preprocess::error::PreprocessError,
    support::source::SourceBuffer,
};
use std::{
    ffi::OsString,
    fs::File,
    io::{self, Write},
    os::unix::ffi::OsStrExt,
    process::ExitCode,
};
// Tool interface vocabulary from support/tool_help_text.h.
const HELP: &str = "usage: cppgm++ --emit-ast -o <outfile> <srcfile>...
   or: cppgm++ --emit-types -o <outfile> <srcfile>...
   or: cppgm++ --emit-semantics -o <outfile> <srcfile>...
   or: cppgm++ --emit-lowir -o <outfile> <srcfile>...
   or: cppgm++ -c [options] <srcfile-or-lowirfile>...
   or: cppgm++ -E [options] <srcfile>...
   or: cppgm++ [options] <input>...

emit-mode flags (every --emit-* mode):
  -I <dir> / -I<dir>  user include search path
  -D <macro> / -D<macro>
  -U <macro> / -U<macro>
  --hosted            the host compiler's include paths and predefined macros, as -c uses
  -O0 / -O1 / -O2 / -O3  (--emit-lowir) the LowIR optimizer
  -g0 / -gline-tables-only  (--emit-lowir) debug locations

query flags:
  --help, -h
  --version
  -v
  -dumpmachine
  -dumpversion
  -print-search-dirs

driver flags:
  -E                  preprocess only
  -c                  compile only
  -o <outfile>        explicit output path
  -I <dir> / -I<dir>  user include search path
  -isystem <dir>      system include search path
  -D <macro> / -D<macro>
  -U <macro> / -U<macro>
  -include <file>
  -L <dir> / -L<dir>
  -l <name> / -l<name>
  -O0 / -O1 / -O2 / -O3 (default: -O0)
  -g0 / -gline-tables-only
  -stdlib <name> / -stdlib=<name>
  --target <target>
";
use cppgm_rust::driver::batch as batch_status;
#[path = "source_tool/build_time.rs"]
mod build_time;
pub mod builtin_host_configuration;
#[path = "source_tool/driver_queries.rs"]
mod driver_queries;
#[path = "source_tool/host_configuration.rs"]
mod host_configuration;
#[path = "source_tool/stack_limit.rs"]
mod stack_limit;
fn source_values() -> Result<cppgm_rust::preprocess::options::PredefinedValues, String> {
    let mut values = build_time::query()?;
    values.author = b"Vish Abrams".to_vec();
    Ok(values)
}
fn peak_rss() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")?
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}
enum CliError {
    Message(String),
    Diagnostic(Vec<u8>),
    ReportedDiagnostic(Vec<u8>),
    PassVerification(Vec<u8>),
    Missing(&'static str),
    Internal(&'static str),
    InternalDiagnostic(Vec<u8>),
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
struct Invocation<'a> {
    output: &'a std::ffi::OsStr,
    inputs: Vec<&'a std::ffi::OsStr>,
    limits: cppgm_rust::opt::Limits,
    preprocessor: cppgm_rust::preprocess::options::DriverOptions,
    level: u8,
    hosted: bool,
    debug: cppgm_rust::driver::debug_options::DebugOptions,
}
fn preprocessing_option(
    args: &[OsString],
    index: &mut usize,
    options: &mut cppgm_rust::preprocess::options::DriverOptions,
) -> Result<bool, CliError> {
    let bytes = args[*index - 1].as_bytes();
    let Some((&option, rest)) = bytes.strip_prefix(b"-").and_then(|s| s.split_first()) else {
        return Ok(false);
    };
    let expected = match option {
        b'I' => "path",
        b'D' => "macro definition",
        b'U' => "macro name",
        _ => return Ok(false),
    };
    let value = if rest.is_empty() {
        let value = args.get(*index).ok_or_else(|| {
            CliError::Message(format!("missing {expected} after -{}", char::from(option)))
        })?;
        *index += 1;
        value.as_bytes()
    } else {
        rest
    };
    match option {
        b'I' => options.include_paths.push(value.to_vec()),
        b'D' => options.define(value),
        b'U' => options.undefine(value),
        _ => return Err(CliError::Internal("preprocessing option")),
    }
    Ok(true)
}
fn invocation(args: &[OsString], mode: EmitMode) -> Result<Invocation<'_>, CliError> {
    let lowir = mode == EmitMode::LowIr;
    let mut output = None;
    let mut inputs = Vec::new();
    let mut limits = cppgm_rust::opt::Limits::default();
    let mut preprocessor = cppgm_rust::preprocess::options::DriverOptions::default();
    let mut level = 0;
    let mut hosted = false;
    let mut debug = cppgm_rust::driver::debug_options::DebugOptions::default();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        index += 1;
        if arg == "--hosted" {
            hosted = true;
            continue;
        }
        if lowir && debug.consume(arg.as_bytes()) {
            continue;
        }
        if lowir && arg.as_bytes().starts_with(b"-O") {
            level = cppgm_rust::driver::optimization_options::OptimizationSyntax::Emit
                .parse(arg.as_bytes())
                .ok_or_else(|| {
                    CliError::Diagnostic([b"unsupported option: ", arg.as_bytes()].concat())
                })?;
            continue;
        }
        if lowir && arg == "--inline-limit" {
            let value = args
                .get(index)
                .ok_or_else(|| CliError::Message("missing limit after --inline-limit".into()))?;
            index += 1;
            if !limits.parse_override(value.as_bytes()) {
                return Err(CliError::Diagnostic(
                    [b"invalid --inline-limit: ".as_slice(), value.as_bytes()].concat(),
                ));
            }
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
        if preprocessing_option(args, &mut index, &mut preprocessor)? {
            continue;
        }
        if lowir && matches!(arg.as_bytes(), b"--witness" | b"--witness-debug") {
            if args.get(index).is_none() {
                return Err(CliError::Diagnostic(
                    [b"missing output file after ", arg.as_bytes()].concat(),
                ));
            }
            index += 1;
            continue;
        }
        if matches!(
            arg.as_bytes(),
            b"-c"
                | b"-E"
                | b"--version"
                | b"-v"
                | b"-dumpmachine"
                | b"-dumpversion"
                | b"-print-search-dirs"
        ) {
            return Err(CliError::Message("invalid usage".to_owned()));
        }
        if arg.as_bytes().starts_with(b"-") {
            return Err(CliError::Diagnostic(
                [b"unsupported option in emit mode: ", arg.as_bytes()].concat(),
            ));
        }
        inputs.push(arg.as_os_str());
    }
    let output = output.ok_or_else(|| CliError::Message("invalid usage".to_owned()))?;
    if inputs.is_empty() {
        return Err(CliError::Message("invalid usage".to_owned()));
    }
    Ok(Invocation {
        output,
        inputs,
        limits,
        preprocessor,
        level,
        hosted,
        debug,
    })
}
fn run() -> Result<(), CliError> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.iter().any(|arg| arg == "--batch-stdin") {
        batch_status::write(&mut io::stdin().lock(), &mut io::stdout().lock());
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        let _status = io::stdout().lock().write_all(HELP.as_bytes());
        return Ok(());
    }
    if !args.iter().any(|arg| {
        matches!(
            arg.as_bytes(),
            b"--emit-ast" | b"--emit-types" | b"--emit-semantics" | b"--emit-lowir"
        )
    }) {
        use std::os::unix::fs::PermissionsExt;
        if driver_queries::run(&args)? {
            return Ok(());
        }
        let values = source_values().map_err(CliError::Message)?;
        let budget = cppgm_rust::support::stack::StackBudget::from_limit(stack_limit::query())
            .map_err(CliError::Internal)?;
        let host = host_configuration::query();
        return cppgm_rust::driver::toolchain::run(
            &args,
            &values,
            budget,
            &std::fs::Permissions::from_mode(0o755),
            &cppgm_rust::driver::host_environment::HostEnvironment {
                strict_predefined_macros: &host.strict_macros,
                predefined_macros: &host.macros,
                include_paths: &host.includes,
                stdlib_flags: builtin_host_configuration::FLAGS,
            },
            peak_rss,
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
    // The frozen parse_emit_mode removes every emit flag before parsing
    // operand-taking options, even when a flag follows -I, -D, -U or -o.
    args.retain(|arg| {
        !matches!(
            arg.as_bytes(),
            b"--emit-ast" | b"--emit-types" | b"--emit-semantics" | b"--emit-lowir"
        )
    });
    let statistics = (lowir || ast || types || semantics) && consume_flag(&mut args, b"--stats");
    let validate = lowir && consume_flag(&mut args, b"--validate");
    let Invocation {
        output: destination,
        inputs,
        limits,
        preprocessor: mut preprocessor_options,
        level,
        hosted,
        debug,
    } = invocation(&args, mode)?;
    let values = source_values().map_err(CliError::Message)?;
    let stack_budget = cppgm_rust::support::stack::StackBudget::from_limit(stack_limit::query())
        .map_err(CliError::Internal)?;
    if lowir || hosted {
        let host = host_configuration::query();
        cppgm_rust::driver::host_environment::HostEnvironment {
            strict_predefined_macros: &host.strict_macros,
            predefined_macros: &host.macros,
            include_paths: &host.includes,
            stdlib_flags: builtin_host_configuration::FLAGS,
        }
        .apply(&mut preprocessor_options, false);
    }
    if level > 0 {
        preprocessor_options
            .command_line
            .splice(..0, b"#define __OPTIMIZE__ 1\n".iter().copied());
    }
    let file = if lowir {
        None
    } else {
        File::create(destination).ok().map(io::BufWriter::new)
    };
    let failed = !lowir && file.is_none();
    let mut output = Output { file, failed };
    let host_object = lowir && debug.level_stated && destination.as_bytes().ends_with(b".lowir");
    let mut lowir_output = if level > 0 || validate {
        LowIrOutput::for_optimization(host_object).map_err(tool_error)?
    } else if host_object {
        LowIrOutput::host_object_view()
    } else {
        LowIrOutput::default()
    };
    lowir_output.set_debug_locations(debug.locations && !host_object);
    if !lowir {
        writeln!(output, "{} translation units", inputs.len()).map_err(|_| CliError::Output)?;
    }
    for (index, path) in inputs.iter().enumerate() {
        if !std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
            return Err(path_error(path, b"cannot open the source file"));
        }
        let bytes = cppgm_rust::driver::file_identity::read(path.as_bytes())
            .map_err(|error| CliError::Message(error.to_string()))?
            .ok_or_else(|| path_error(path, b"cannot open the source file"))?;
        let source =
            SourceBuffer::new(bytes).map_err(|message| CliError::Message(message.to_owned()))?;
        let mut source_observations = cppgm_rust::driver::ast::SourceObservations::default();
        let result = emit_configured_unit(
            &source,
            path.as_bytes(),
            UnitOptions {
                values: &values,
                preprocessor: &preprocessor_options,
                stack_budget,
            },
            if lowir {
                Destination::LowIr(&mut lowir_output)
            } else if statistics {
                Destination::ObservedText(&mut output, &mut source_observations)
            } else {
                Destination::Text(&mut output)
            },
            index + 1,
            mode,
        )
        .map_err(|error| match error {
            AstToolError::PassVerification(bytes) => CliError::PassVerification(bytes),
            AstToolError::InternalDiagnostic(bytes) => CliError::InternalDiagnostic(bytes),
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
            AstToolError::Initialization(PreprocessError::Read(error)) => {
                CliError::Message(error.to_string())
            }
            AstToolError::Initialization(error) => {
                let mut bytes = path.as_bytes().to_vec();
                bytes.push(b':');
                bytes.extend_from_slice(
                    cppgm_rust::driver::preproc::describe(&source, &error).as_bytes(),
                );
                CliError::Diagnostic(bytes)
            }
        });
        if statistics && !lowir {
            if let Err(CliError::Diagnostic(diagnostic)) = result {
                let mut bytes = b"ERROR: ".to_vec();
                bytes.extend(diagnostic);
                bytes.push(b'\n');
                source_observations
                    .write(&mut bytes, path.as_bytes(), peak_rss())
                    .map_err(|_| CliError::Output)?;
                return Err(CliError::ReportedDiagnostic(bytes));
            }
            if result.is_ok() {
                let _status = source_observations.write(
                    &mut io::stderr().lock(),
                    path.as_bytes(),
                    peak_rss(),
                );
            }
        }
        result?;
    }
    let mut observations = None;
    if lowir {
        let view = lowir_output
            .prepare(level, limits, validate)
            .map_err(tool_error)?;
        output.file = File::create(destination).ok().map(io::BufWriter::new);
        output.failed = output.file.is_none();
        observations = Some(view.write(&mut output).map_err(|_| CliError::Output)?);
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
    if statistics && let Some(observations) = observations {
        let _ = observations.write(&mut io::stderr().lock(), peak_rss());
    }
    Ok(())
}
fn consume_flag(args: &mut Vec<OsString>, flag: &[u8]) -> bool {
    if let Some(index) = args.iter().position(|arg| arg.as_bytes() == flag) {
        args.remove(index);
        true
    } else {
        false
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Output) => ExitCode::FAILURE,
        Err(CliError::ReportedDiagnostic(bytes)) => {
            let _status = io::stderr().lock().write_all(&bytes);
            ExitCode::FAILURE
        }
        Err(CliError::PassVerification(bytes)) => {
            let _ = io::stderr().lock().write_all(&bytes);
            ExitCode::from(3)
        }
        Err(CliError::Missing(group)) => {
            let _ = writeln!(
                io::stderr().lock(),
                "cppgm++: Rust port is not implemented yet: {group}"
            );
            ExitCode::from(86)
        }
        Err(CliError::Internal(message)) => {
            let _ = writeln!(io::stderr().lock(), "internal compiler error: {message}");
            ExitCode::from(70)
        }
        Err(CliError::InternalDiagnostic(bytes)) => {
            let mut out = io::stderr().lock();
            let _ = out.write_all(b"internal compiler error: ");
            let _ = out.write_all(&bytes);
            let _ = out.write_all(b"\n");
            ExitCode::from(70)
        }
        Err(CliError::Message(message)) => {
            let _ = writeln!(io::stderr().lock(), "ERROR: {message}");
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
        AstToolError::Initialization(PreprocessError::Read(error)) => {
            CliError::Message(error.to_string())
        }
        AstToolError::InternalDiagnostic(bytes) => CliError::InternalDiagnostic(bytes),
        AstToolError::PassVerification(bytes) => CliError::PassVerification(bytes),
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
