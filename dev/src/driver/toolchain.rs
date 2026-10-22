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
}
#[derive(Default)]
struct Invocation {
    compile: bool,
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
                    "-l" => "library name",
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
fn parse(args: &[OsString]) -> Result<Invocation, AstToolError> {
    let mut invocation = Invocation::default();
    let mut checks = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        let bytes = arg.as_bytes();
        match bytes {
            b"-c" => invocation.compile = true,
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
            b"-E" => return Err(AstToolError::Missing("hosted preprocessing mode")),
            b"--target=" => return Err(message("missing target after --target")),
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
    if invocation
        .inputs
        .iter()
        .all(|input| input.kind == Kind::Library)
    {
        return Err(message("invalid usage"));
    }
    if invocation.compile
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
    // cppgm++.cpp checks the invocation's shape before toolchain.cpp checks
    // option meanings. Preserve that failure precedence without rereading argv.
    for check in checks {
        match check {
            OptionCheck::Target(arg) => target(arg)?,
            OptionCheck::Optimization(arg) => invocation.level = optimization_level(arg)?,
        }
    }
    if invocation.level != 0 {
        return Err(AstToolError::Missing("optimized source objects"));
    }
    Ok(invocation)
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
    if input.kind == Kind::LowIr {
        return Err(AstToolError::Missing("LowIR object input"));
    }
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
    let source = SourceBuffer::new(bytes).map_err(message)?;
    let mut output = LowIrOutput::for_object().map_err(ast::native_error)?;
    ast::emit_compilation_unit(
        &source,
        input.path.as_os_str().as_bytes(),
        values,
        &invocation.preprocessor,
        &mut output,
        budget,
    )
    .map_err(|error| match error {
        AstToolError::Initialization(PreprocessError::NotImplemented(group)) => {
            AstToolError::Missing(group)
        }
        AstToolError::Initialization(error) => AstToolError::Diagnostic(
            [
                input.path.as_os_str().as_bytes(),
                b":",
                super::preproc::describe(&source, &error).as_bytes(),
            ]
            .concat(),
        ),
        error => error,
    })?;
    output.finish_object(input.path.as_os_str().as_bytes().to_vec())
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
) -> Result<(), AstToolError> {
    let invocation = parse(args)?;
    if invocation.compile {
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
            if !path.as_os_str().as_bytes().ends_with(b".obj") {
                return Err(AstToolError::Missing("host relocatable object emission"));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invocation_shape_precedes_option_meaning() {
        let cases: &[(&[&str], &[u8])] = &[
            (&["--target=invalid"], b"invalid usage"),
            (&["-Oinvalid"], b"invalid usage"),
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
}
