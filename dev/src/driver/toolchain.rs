//! Compile and link orchestration, translated from driver/toolchain.cpp. See NOTICE.
use super::ast::{self, AstToolError, LowIrOutput};
use crate::{
    native,
    preprocess::{
        error::PreprocessError,
        options::{DriverOptions, PredefinedValues},
    },
    support::{source::SourceBuffer, stack::StackBudget},
};
use std::{
    ffi::{OsStr, OsString},
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::MetadataExt,
    },
    path::{Path, PathBuf},
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Source,
    LowIr,
    Object,
    Library,
}
struct Input {
    kind: Kind,
    path: PathBuf,
}
enum OptionCheck<'a> {
    Target(&'a OsStr),
    Optimization(&'a OsStr),
    Standard(&'a OsStr),
    Language(&'a OsStr),
    Compatibility(&'a OsStr),
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum DriverMode {
    #[default]
    Link,
    Compile,
    Preprocess,
}
#[derive(Default)]
struct Invocation {
    nostdinc: bool,
    syntax_only: bool,
    mode: DriverMode,
    explicit_output: bool,
    output: PathBuf,
    inputs: Vec<Input>,
    libraries: Vec<PathBuf>,
    preprocessor: DriverOptions,
    level: u8,
}
fn message(s: &str) -> AstToolError {
    AstToolError::Diagnostic(s.as_bytes().to_vec())
}
fn argument(prefix: &str, arg: &OsStr) -> AstToolError {
    AstToolError::Diagnostic([prefix.as_bytes(), arg.as_bytes()].concat())
}
fn value<'a>(
    args: &'a [OsString],
    index: &mut usize,
    option: &str,
) -> Result<Option<&'a OsStr>, AstToolError> {
    let arg = &args[*index];
    if arg == option {
        *index += 1;
        return args
            .get(*index)
            .map(|v| Some(v.as_os_str()))
            .ok_or_else(|| {
                let role = match option {
                    "-D" => "macro definition",
                    "-U" => "macro name",
                    "-l" => "library name",
                    "-MF" => "depfile path",
                    "-MT" | "-MQ" => "target",
                    _ => "path",
                };
                message(&format!("missing {role} after {option}"))
            });
    }
    Ok(arg
        .as_bytes()
        .strip_prefix(option.as_bytes())
        .filter(|v| !v.is_empty())
        .map(OsStr::from_bytes))
}
fn target(arg: &OsStr) -> Result<(), AstToolError> {
    let t = arg.as_bytes();
    if t == b"linux" {
        return Ok(());
    }
    if let Some(rest) = t.strip_prefix(b"x86_64-")
        && let Some(at) = rest.windows(5).position(|s| s == b"linux")
    {
        let end = at + 5;
        if (at == 0 || rest[..at].iter().position(|&b| b == b'-') == Some(at - 1))
            && (end == rest.len() || rest[end] == b'-')
        {
            return Ok(());
        }
    }
    Err(argument("unsupported target: ", arg))
}
fn parse(
    args: &[OsString],
    host: &super::host_environment::HostEnvironment<'_>,
) -> Result<Invocation, AstToolError> {
    let mut invocation = Invocation::default();
    let mut checks = Vec::new();
    let mut compile_only = false;
    let mut preprocess_only = false;
    let mut forced_includes = Vec::new();
    let mut pthread = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        let bytes = arg.as_bytes();
        match bytes {
            b"-nostdinc" => invocation.nostdinc = true,
            b"-c" => compile_only = true,
            b"-fsyntax-only" => invocation.syntax_only = true,
            b"-pthread" => pthread = true,
            b"-fno-exceptions" | b"-fexceptions" => {
                invocation.preprocessor.modes.exceptions = bytes == b"-fexceptions";
            }
            b"-fno-rtti" | b"-frtti" => invocation.preprocessor.modes.rtti = bytes == b"-frtti",
            b"-x" => {
                index += 1;
                let language = args
                    .get(index)
                    .ok_or_else(|| message("missing language after -x"))?;
                checks.push(OptionCheck::Language(language));
            }
            b"-o" => {
                invocation.explicit_output = true;
                index += 1;
                invocation.output = args
                    .get(index)
                    .ok_or_else(|| message("missing output file after -o"))?
                    .into();
            }
            b"--target" => {
                index += 1;
                checks.push(OptionCheck::Target(
                    args.get(index)
                        .ok_or_else(|| message("missing target after --target"))?,
                ));
            }
            b"-pedantic"
            | b"-pedantic-errors"
            | b"-g0"
            | b"-fvisibility=hidden"
            | b"-fvisibility-inlines-hidden" => {}
            b"-E" => preprocess_only = true,
            b"-std" | b"-stdlib" => {
                let role = if bytes == b"-std" {
                    "language standard"
                } else {
                    "standard library name"
                };
                index += 1;
                if args.get(index).is_none() {
                    return Err(message(&format!(
                        "missing {role} after {}",
                        arg.to_string_lossy()
                    )));
                }
                checks.push(OptionCheck::Compatibility(arg));
            }
            b"-include" => {
                index += 1;
                let path = args
                    .get(index)
                    .ok_or_else(|| message("missing file after -include"))?;
                forced_includes.extend_from_slice(b"#include \"");
                forced_includes.extend_from_slice(path.as_bytes());
                forced_includes.extend_from_slice(b"\"\n");
            }
            b"--target=" => return Err(message("missing target after --target")),
            _ if bytes.starts_with(b"-std=") => {
                checks.push(OptionCheck::Standard(OsStr::from_bytes(&bytes[5..])));
            }
            _ if bytes.starts_with(b"-stdlib=") => {
                checks.push(OptionCheck::Compatibility(arg));
            }
            _ if bytes.starts_with(b"-MF")
                || bytes.starts_with(b"-MT")
                || bytes.starts_with(b"-MQ") =>
            {
                let option = std::str::from_utf8(&bytes[..3])
                    .map_err(|_| AstToolError::Internal("dependency option"))?;
                let _ = value(args, &mut index, option)?;
            }
            _ if bytes.starts_with(b"--target=") => {
                checks.push(OptionCheck::Target(OsStr::from_bytes(&bytes[9..])));
            }
            _ if bytes.starts_with(b"-O") => {
                checks.push(OptionCheck::Optimization(arg));
            }
            _ if bytes.starts_with(b"-I") => {
                let dir = value(args, &mut index, "-I")?
                    .ok_or(AstToolError::Internal("include argument"))?;
                invocation
                    .preprocessor
                    .include_paths
                    .push(dir.as_bytes().to_vec());
            }
            _ if bytes.starts_with(b"-isystem") => {
                let dir = value(args, &mut index, "-isystem")?
                    .ok_or(AstToolError::Internal("system include argument"))?;
                invocation
                    .preprocessor
                    .system_include_paths
                    .push(dir.as_bytes().to_vec());
            }
            _ if bytes.starts_with(b"-D") => {
                let define = value(args, &mut index, "-D")?
                    .ok_or(AstToolError::Internal("definition argument"))?
                    .as_bytes();
                let at = define.iter().position(|&b| b == b'=');
                let (name, replacement) = at.map_or((define, b"1".as_slice()), |at| {
                    (&define[..at], &define[at + 1..])
                });
                invocation
                    .preprocessor
                    .command_line
                    .extend_from_slice(b"#define ");
                invocation.preprocessor.command_line.extend_from_slice(name);
                invocation.preprocessor.command_line.push(b' ');
                invocation
                    .preprocessor
                    .command_line
                    .extend_from_slice(replacement);
                invocation.preprocessor.command_line.push(b'\n');
            }
            _ if bytes.starts_with(b"-U") => {
                let name = value(args, &mut index, "-U")?
                    .ok_or(AstToolError::Internal("undefinition argument"))?;
                invocation
                    .preprocessor
                    .command_line
                    .extend_from_slice(b"#undef ");
                invocation
                    .preprocessor
                    .command_line
                    .extend_from_slice(name.as_bytes());
                invocation.preprocessor.command_line.push(b'\n');
            }
            _ if bytes.starts_with(b"-L") => {
                let path = value(args, &mut index, "-L")?
                    .ok_or(AstToolError::Internal("library path argument"))?;
                invocation.libraries.push(path.into());
            }
            _ if bytes.starts_with(b"-l") => {
                let path = value(args, &mut index, "-l")?
                    .ok_or(AstToolError::Internal("library argument"))?;
                invocation.inputs.push(Input {
                    kind: Kind::Library,
                    path: path.into(),
                });
            }
            _ if bytes.starts_with(b"-") => {
                if super::compatibility::outer_flag(bytes) {
                    checks.push(OptionCheck::Compatibility(arg));
                    index += 1;
                    continue;
                }
                return Err(argument("unsupported driver option: ", arg));
            }
            _ => invocation.inputs.push(Input {
                kind: if bytes.ends_with(b".o") || bytes.ends_with(b".obj") {
                    Kind::Object
                } else if bytes.ends_with(b".lowir") {
                    Kind::LowIr
                } else {
                    Kind::Source
                },
                path: arg.into(),
            }),
        }
        index += 1;
    }
    if compile_only && preprocess_only {
        return Err(message("cannot combine -c and -E"));
    }
    if invocation
        .inputs
        .iter()
        .all(|input| input.kind == Kind::Library)
    {
        return Err(message("invalid usage"));
    }
    if (compile_only || preprocess_only)
        && invocation.explicit_output
        && invocation
            .inputs
            .iter()
            .filter(|i| i.kind != Kind::Library)
            .count()
            != 1
    {
        return Err(message(
            "cannot specify -o when generating multiple output files",
        ));
    }
    invocation.mode = if preprocess_only {
        DriverMode::Preprocess
    } else if compile_only {
        DriverMode::Compile
    } else {
        DriverMode::Link
    };
    // cppgm++.cpp checks the invocation's shape before toolchain.cpp checks
    // option meanings. Preserve that failure precedence without rereading argv.
    for check in checks {
        match check {
            OptionCheck::Target(arg) => target(arg)?,
            OptionCheck::Language(language) => {
                if language != "c++" {
                    return Err(argument("unsupported language: ", language));
                }
            }
            OptionCheck::Standard(name) => {
                let (standard, strict) =
                    crate::preprocess::options::LanguageStandard::parse(name.as_bytes())
                        .ok_or_else(|| argument("unsupported language standard: ", name))?;
                invocation.preprocessor.standard = standard;
                invocation.preprocessor.strict = strict;
            }
            OptionCheck::Optimization(arg) => invocation.level = optimization_level(arg)?,
            OptionCheck::Compatibility(arg) => super::compatibility::validate(arg, host)?,
        }
    }
    if pthread {
        invocation
            .preprocessor
            .command_line
            .splice(..0, b"#define _REENTRANT 1\n".iter().copied());
    }
    if invocation.level > 0 {
        invocation
            .preprocessor
            .command_line
            .splice(..0, b"#define __OPTIMIZE__ 1\n".iter().copied());
    }
    invocation.preprocessor.command_line.extend(forced_includes);
    prune_include_paths(&mut invocation.preprocessor);
    Ok(invocation)
}
// A directory shared with the system chain is ignored in the user chain.
// Follow stat identity, including aliases, while retaining first spellings and
// nonexistent/non-directory entries just as the frozen driver does.
fn prune_include_paths(options: &mut DriverOptions) {
    let mut seen = std::collections::HashSet::new();
    for paths in [
        &mut options.system_include_paths,
        &mut options.include_paths,
    ] {
        paths.retain(|path| {
            std::fs::metadata(OsStr::from_bytes(path)).map_or(true, |metadata| {
                !metadata.is_dir() || seen.insert((metadata.dev(), metadata.ino()))
            })
        });
    }
}
fn optimization_level(arg: &OsStr) -> Result<u8, AstToolError> {
    let text = &arg.as_bytes()[2..];
    match text {
        b"" | b"g" => Ok(1),
        b"s" | b"z" => Ok(2),
        b"fast" => Ok(3),
        _ if !text.is_empty() && text.iter().all(u8::is_ascii_digit) => Ok(text
            .iter()
            .fold(0_u8, |n, b| n.saturating_mul(10).saturating_add(b - b'0'))
            .min(3)),
        _ => Err(argument("unsupported option: ", arg)),
    }
}
fn check_output(invocation: &Invocation, path: &Path) -> Result<(), AstToolError> {
    if let Ok(out) = std::fs::metadata(path) {
        for input in &invocation.inputs {
            if input.kind != Kind::Library
                && std::fs::metadata(&input.path)
                    .is_ok_and(|meta| meta.dev() == out.dev() && meta.ino() == out.ino())
            {
                return Err(AstToolError::Diagnostic(
                    [
                        b"input file ",
                        input.path.as_os_str().as_bytes(),
                        b" is the same as the output file",
                    ]
                    .concat(),
                ));
            }
        }
    }
    Ok(())
}
fn compile(
    input: &Input,
    invocation: &Invocation,
    values: &PredefinedValues,
    budget: StackBudget,
) -> Result<native::Object, AstToolError> {
    let host = invocation.mode == DriverMode::Compile
        && !invocation.output.as_os_str().as_bytes().ends_with(b".obj");
    let model = if host {
        native::ObjectModel::Host
    } else {
        native::ObjectModel::Course
    };
    if input.kind == Kind::LowIr {
        let mut program = crate::lowir::read_object_file(&input.path).map_err(ast::native_error)?;
        native::recover_jump_thunks(&mut program, model);
        if invocation.level != 0 {
            crate::opt::optimize(&mut program, invocation.level).map_err(ast::native_error)?;
            program.clear_jump_thunks();
            native::recover_jump_thunks(&mut program, model);
        }
        program
            .set_definition_sink(Box::new(
                native::ObjectBuilder::with_model(invocation.level, model)
                    .map_err(ast::native_error)?,
            ))
            .map_err(ast::native_error)?;
        return native::compile_object(
            &mut program,
            input.path.as_os_str().as_bytes().to_vec(),
            invocation.level,
        )
        .map_err(ast::native_error);
    }
    let source = read_source(input)?;
    let mut output =
        LowIrOutput::for_object_level(model, invocation.level).map_err(ast::native_error)?;
    ast::emit_compilation_unit(
        &source,
        input.path.as_os_str().as_bytes(),
        values,
        &invocation.preprocessor,
        &mut output,
        budget,
    )
    .map_err(|error| source_error(input, &source, error))?;
    // Optimization owns typed LowIR alone; source storage ends with the front
    // end, before the retained unit reaches its optimizer/backend consumers.
    drop(source);
    output
        .finish_object(input.path.as_os_str().as_bytes().to_vec())
        .map_err(|error| match error {
            AstToolError::NativeDiagnostic(bytes) => AstToolError::Diagnostic(
                [input.path.as_os_str().as_bytes(), b": ", &bytes].concat(),
            ),
            error => error,
        })
}
fn read_source(input: &Input) -> Result<SourceBuffer, AstToolError> {
    if !std::fs::metadata(&input.path).is_ok_and(|m| m.is_file()) {
        return Err(AstToolError::Diagnostic(
            [
                input.path.as_os_str().as_bytes(),
                b": cannot open the source file",
            ]
            .concat(),
        ));
    }
    let bytes = std::fs::read(&input.path).map_err(|_| {
        AstToolError::Diagnostic(
            [
                input.path.as_os_str().as_bytes(),
                b": cannot open the source file",
            ]
            .concat(),
        )
    })?;
    SourceBuffer::new(bytes).map_err(message)
}
fn source_error(input: &Input, source: &SourceBuffer, error: AstToolError) -> AstToolError {
    match error {
        AstToolError::Initialization(PreprocessError::NotImplemented(group)) => {
            AstToolError::Missing(group)
        }
        AstToolError::Initialization(error) => AstToolError::Diagnostic(
            [
                input.path.as_os_str().as_bytes(),
                b":",
                super::preproc::describe(source, &error).as_bytes(),
            ]
            .concat(),
        ),
        AstToolError::NativeDiagnostic(bytes) => {
            AstToolError::Diagnostic([input.path.as_os_str().as_bytes(), b": ", &bytes].concat())
        }
        error => error,
    }
}
fn check_source(
    input: &Input,
    invocation: &Invocation,
    values: &PredefinedValues,
    budget: StackBudget,
) -> Result<(), AstToolError> {
    let source = read_source(input)?;
    ast::emit_configured_unit(
        &source,
        input.path.as_os_str().as_bytes(),
        ast::UnitOptions {
            values,
            preprocessor: &invocation.preprocessor,
            stack_budget: budget,
        },
        ast::Destination::Check,
        1,
        ast::EmitMode::Semantics,
    )
    .map_err(|error| source_error(input, &source, error))
}
fn library(invocation: &Invocation, name: &Path) -> Result<PathBuf, AstToolError> {
    for dir in &invocation.libraries {
        for suffix in [b".o".as_slice(), b".obj"] {
            let file = OsString::from_vec([b"lib", name.as_os_str().as_bytes(), suffix].concat());
            let path = PathBuf::from(OsString::from_vec(
                [
                    dir.as_os_str().as_bytes(),
                    if dir.as_os_str().as_bytes().ends_with(b"/") {
                        b"".as_slice()
                    } else {
                        b"/".as_slice()
                    },
                    file.as_bytes(),
                ]
                .concat(),
            ));
            if std::fs::metadata(&path).is_ok_and(|m| m.is_file()) {
                return Ok(path);
            }
        }
    }
    Err(argument("library not found: -l", name.as_os_str()))
}
/// Compile each source independently and link in command-line input order.
/// # Errors
/// Returns a source, object, driver or native diagnostic, or an unfinished group.
pub fn run(
    args: &[OsString],
    values: &PredefinedValues,
    budget: StackBudget,
    permissions: &std::fs::Permissions,
    host: &super::host_environment::HostEnvironment<'_>,
) -> Result<(), AstToolError> {
    let mut invocation = parse(args, host)?;
    host.apply(&mut invocation.preprocessor, invocation.nostdinc);
    prune_include_paths(&mut invocation.preprocessor);
    if invocation.mode == DriverMode::Preprocess {
        return preprocess(&invocation, values);
    }
    if invocation.syntax_only {
        for input in &invocation.inputs {
            if input.kind == Kind::Source {
                check_source(input, &invocation, values, budget)?;
            }
        }
        return Ok(());
    }
    if invocation.mode == DriverMode::Compile {
        for input in &invocation.inputs {
            if input.kind == Kind::Library {
                continue;
            }
            if input.kind == Kind::Object {
                return Err(argument(
                    "-c needs C++ source or LowIR inputs: ",
                    input.path.as_os_str(),
                ));
            }
            let path = if invocation.output.as_os_str().is_empty() {
                let name = input.path.file_name().unwrap_or(input.path.as_os_str());
                let bytes = name.as_bytes();
                let at = bytes
                    .iter()
                    .rposition(|&b| b == b'.')
                    .filter(|&n| n != 0)
                    .unwrap_or(bytes.len());
                PathBuf::from(OsString::from_vec([&bytes[..at], b".o"].concat()))
            } else {
                invocation.output.clone()
            };
            let object = compile(input, &invocation, values, budget)?;
            check_output(&invocation, &path)?;
            native::write_object(&object, &path).map_err(ast::native_error)?;
        }
        return Ok(());
    }
    let path = if invocation.output.as_os_str().is_empty() {
        Path::new("a.out")
    } else {
        &invocation.output
    };
    check_output(&invocation, path)?;
    let mut objects = Vec::with_capacity(invocation.inputs.len());
    for input in &invocation.inputs {
        objects.push(match input.kind {
            Kind::Source | Kind::LowIr => compile(input, &invocation, values, budget)?,
            Kind::Object => native::read_object(&input.path).map_err(ast::native_error)?,
            Kind::Library => native::read_object(&library(&invocation, &input.path)?)
                .map_err(ast::native_error)?,
        });
    }
    native::link_objects(objects, path, Some(permissions)).map_err(|error| {
        let _ = std::fs::remove_file(path);
        ast::native_error(error)
    })
}

// The output opens before the first source; every unit writes sof even if
// source construction fails. Borrow one source at a time and stream its tokens.
fn preprocess(invocation: &Invocation, values: &PredefinedValues) -> Result<(), AstToolError> {
    use std::io::Write;
    for input in &invocation.inputs {
        if matches!(input.kind, Kind::Object | Kind::LowIr) {
            return Err(argument(
                "-E needs C++ source inputs: ",
                input.path.as_os_str(),
            ));
        }
    }
    let sources: Vec<_> = invocation
        .inputs
        .iter()
        .filter(|i| i.kind == Kind::Source)
        .collect();
    if sources.len() != 1 && !invocation.output.as_os_str().is_empty() {
        return Err(message("cannot specify -o with -E and several inputs"));
    }
    let writer: Box<dyn Write> = if invocation.output.as_os_str().is_empty() {
        Box::new(std::io::stdout().lock())
    } else {
        check_output(invocation, &invocation.output)?;
        Box::new(
            std::fs::File::create(&invocation.output)
                .map_err(|_| argument("cannot write ", invocation.output.as_os_str()))?,
        )
    };
    let mut output = super::preproc::HostedOutput::new(writer);
    writeln!(output, "preproc {}", sources.len()).map_err(AstToolError::Output)?;
    for input in sources {
        output
            .write_all(b"sof ")
            .and_then(|()| output.write_all(input.path.as_os_str().as_bytes()))
            .and_then(|()| output.write_all(b"\n"))
            .map_err(AstToolError::Output)?;
        let bytes = if std::fs::metadata(&input.path).is_ok_and(|m| m.is_file()) {
            std::fs::read(&input.path).ok()
        } else {
            None
        }
        .ok_or_else(|| argument_source(&input.path, b"cannot open the source file"))?;
        let source = SourceBuffer::new(bytes).map_err(message)?;
        super::preproc::emit_hosted(
            &source,
            input.path.as_os_str().as_bytes(),
            values,
            &invocation.preprocessor,
            &mut output,
        )
        .map_err(|error| match error {
            super::preproc::PreprocToolError::Initialization(PreprocessError::NotImplemented(
                group,
            ))
            | super::preproc::PreprocToolError::Preprocess {
                error: PreprocessError::NotImplemented(group),
                ..
            } => AstToolError::Missing(group),
            super::preproc::PreprocToolError::Initialization(PreprocessError::Invariant(
                message,
            ))
            | super::preproc::PreprocToolError::Preprocess {
                error: PreprocessError::Invariant(message),
                ..
            } => AstToolError::Internal(message),
            super::preproc::PreprocToolError::Initialization(PreprocessError::Source(error)) => {
                AstToolError::Diagnostic(error.to_string().into_bytes())
            }
            super::preproc::PreprocToolError::Initialization(error) => argument_source(
                &input.path,
                super::preproc::describe(&source, &error).as_bytes(),
            ),
            super::preproc::PreprocToolError::Preprocess { diagnostic, .. } => {
                AstToolError::Diagnostic(diagnostic)
            }
            super::preproc::PreprocToolError::InvalidToken => {
                argument_source(&input.path, b"invalid phase-7 token")
            }
            super::preproc::PreprocToolError::Output(error) => AstToolError::Output(error),
        })?;
    }
    output.flush().map_err(AstToolError::Output)?;
    output.finish().map_err(AstToolError::Output)
}
fn argument_source(path: &Path, diagnostic: &[u8]) -> AstToolError {
    AstToolError::Diagnostic([path.as_os_str().as_bytes(), b": ", diagnostic].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[OsString]) -> Result<Invocation, AstToolError> {
        super::parse(
            args,
            &super::super::host_environment::HostEnvironment {
                strict_predefined_macros: b"",
                predefined_macros: b"",
                include_paths: b"/usr/include/c++/15",
                stdlib_flags: b"",
            },
        )
    }

    #[test]
    fn preprocessing_keeps_command_order_and_defers_forced_includes() {
        let args = [
            "-E",
            "-include",
            "header",
            "-DNAME=1",
            "-pthread",
            "-UNAME",
            "-DNAME=2",
            "-std=c++17",
            "-fno-exceptions",
            "-O2",
            "-x",
            "c++",
            "source",
        ];
        let args: Vec<_> = args.iter().map(OsString::from).collect();
        let invocation = parse(&args).unwrap();
        assert!(invocation.mode == DriverMode::Preprocess);
        assert_eq!(invocation.preprocessor.command_line, b"#define __OPTIMIZE__ 1\n#define _REENTRANT 1\n#define NAME 1\n#undef NAME\n#define NAME 2\n#include \"header\"\n");
        assert!(invocation.preprocessor.strict);
        assert!(!invocation.preprocessor.modes.exceptions);
        assert_eq!(invocation.preprocessor.standard.macro_value(), b"201703L");
        assert_eq!(invocation.inputs.len(), 1);
    }

    #[test]
    fn invocation_shape_precedes_option_meaning() {
        let cases: &[(&[&str], &[u8])] = &[
            (&["--target=invalid"], b"invalid usage"),
            (&["-Oinvalid"], b"invalid usage"),
            (&["-std=invalid"], b"invalid usage"),
            (&["-x", "c"], b"invalid usage"),
            (&["-E", "-c"], b"cannot combine -c and -E"),
            (&["-E", "-U"], b"missing macro name after -U"),
            (&["--target="], b"missing target after --target"),
            (&["--target=invalid", "-I"], b"missing path after -I"),
            (
                &["-c", "--target=invalid", "-o", "out.obj", "a.cpp", "b.cpp"],
                b"cannot specify -o when generating multiple output files",
            ),
            (
                &["--target=invalid", "a.cpp"],
                b"unsupported target: invalid",
            ),
            (&["-Oinvalid", "a.cpp"], b"unsupported option: -Oinvalid"),
            (&["-fshort-enums"], b"invalid usage"),
            (
                &["-stdlib=libc++", "-MF"],
                b"missing depfile path after -MF",
            ),
            (&["-funsigned-char", "-MQ"], b"missing target after -MQ"),
            (&["-std"], b"missing language standard after -std"),
            (&["-std", "c++14", "a.cpp"], b"unsupported option: -std"),
            (&["-pg", "a.cpp"], b"unsupported option: -pg"),
            (
                &["-m32", "a.cpp"],
                b"unsupported option (it changes the program's meaning): -m32",
            ),
            (
                &["-stdlib=libc++", "a.cpp"],
                b"this compiler was not built for -stdlib=libc++ (CPPGM_STDLIB_FLAGS)",
            ),
        ];
        for &(args, expected) in cases {
            let args: Vec<_> = args.iter().map(OsString::from).collect();
            assert!(
                matches!(parse(&args), Err(AstToolError::Diagnostic(actual))
                if actual == expected),
                "{args:?}"
            );
        }
    }

    #[test]
    fn hosted_flags_keep_dependency_operands_out_of_the_source_list() {
        let args: Vec<_> = [
            "-c",
            "-Wall",
            "-Wextra",
            "-pipe",
            "-w",
            "-fPIC",
            "-m64",
            "-MMD",
            "-MP",
            "-MF",
            "deps.d",
            "-MTtarget",
            "-MQ",
            "quoted target",
            "-stdlib=libstdc++",
            "-fsyntax-only",
            "source.cpp",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        let invocation = parse(&args).unwrap();
        assert!(invocation.syntax_only);
        assert_eq!(invocation.inputs.len(), 1);
        assert_eq!(invocation.inputs[0].path, Path::new("source.cpp"));
        assert!(invocation.preprocessor.command_line.is_empty());
    }
}
