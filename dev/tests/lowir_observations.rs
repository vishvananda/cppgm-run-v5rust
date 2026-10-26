//! Constructed `LowIR` validation and work reports preserve the output view.
use std::{collections::HashMap, path::PathBuf, process::Command};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn failed_statistics_sink_preserves_success_and_completed_tool_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(std::env::temp_dir().join(format!(
        "cppgm-lowir-statistics-sink-{}",
        std::process::id()
    )));
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.lowir");
    let output = directory.0.join("output");
    std::fs::write(
        &source,
        b"function @main() -> i64 [role=entry] { block ^entry: return i64 0 }\n",
    )?;
    for (tool, flags) in [
        (env!("CARGO_BIN_EXE_lowir"), &["-o"][..]),
        (env!("CARGO_BIN_EXE_lowiropt"), &["-O0", "-o"][..]),
        (
            env!("CARGO_BIN_EXE_lowir2native"),
            &["--dump-machine-ir"][..],
        ),
    ] {
        let mut command = Command::new(tool);
        command.args(flags).arg(&output).arg(&source);
        let plain = command.output()?;
        assert!(plain.status.success(), "{tool}: {:?}", plain.stderr);
        assert!(plain.stdout.is_empty());
        assert!(plain.stderr.is_empty());
        let expected = std::fs::read(&output)?;
        let reported = command
            .arg("--stats")
            .stderr(std::fs::OpenOptions::new().write(true).open("/dev/full")?)
            .output()?;
        assert!(reported.status.success(), "{tool}: {:?}", reported.status);
        assert!(reported.stdout.is_empty());
        assert_eq!(std::fs::read(&output)?, expected);
    }
    Ok(())
}
#[test]
fn streamed_and_retained_reports_agree_without_changing_lowir()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-lowir-observations-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    std::fs::write(&source,b"constexpr int twice(int value){return value+value;}\nconstexpr int number=twice(3);\nconst int other=5;\nint main(){return number+other;}\n")?;
    for extension in ["lowir", "view"] {
        for level in 0..=3 {
            let path = directory.0.join(format!("output.{extension}"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_cppgm"));
            command
                .args(["--emit-lowir", "-g0", &format!("-O{level}"), "-o"])
                .arg(&path)
                .arg(&source);
            let baseline = command.output()?;
            assert!(baseline.status.success(), "{:?}", baseline.stderr);
            assert!(baseline.stderr.is_empty());
            let bytes = std::fs::read(&path)?;
            command.arg("--stats");
            let mut reports = Vec::new();
            for validate in [false, true] {
                if validate {
                    command.arg("--validate");
                }
                let observed = command.output()?;
                assert!(observed.status.success(), "{:?}", observed.stderr);
                assert!(observed.stdout.is_empty());
                assert_eq!(bytes, std::fs::read(&path)?);
                let stderr = std::str::from_utf8(&observed.stderr)?;
                let mut lines = stderr.lines();
                let lowering = lines.next().ok_or("missing lowering report")?;
                assert!(lowering.starts_with("cppgm++ lowir stats: "));
                let fields: HashMap<_, _> = lowering
                    .split_whitespace()
                    .filter_map(|field| field.split_once('='))
                    .filter(|(key, _)| {
                        !key.ends_with("_ms")
                            && !matches!(*key, "peak_rss_kb" | "lowir_arena_bytes")
                    })
                    .collect();
                assert_eq!(fields.get("constant_steps"), Some(&"16"));
                assert_eq!(fields.get("names"), Some(&"14"));
                reports.push(
                    fields
                        .into_iter()
                        .map(|(key, value)| (key.to_owned(), value.to_owned()))
                        .collect::<HashMap<_, _>>(),
                );
                assert_eq!(
                    lines
                        .next()
                        .is_some_and(|line| line.starts_with("pa32_opt_stats ")),
                    level != 0
                );
                assert!(lines.next().is_none());
            }
            assert_eq!(reports[0], reports[1]);
        }
    }
    Ok(())
}

#[test]
fn merged_units_publish_resolved_globals_once() -> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-merged-observations-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let reference = directory.0.join("reference.cpp");
    let definition = directory.0.join("definition.cpp");
    std::fs::write(
        &reference,
        b"extern int shared; int read(){return shared;}\n",
    )?;
    std::fs::write(&definition, b"int shared=7;\n")?;
    for level in 0..=3 {
        for validate in [false, true] {
            let output = directory.0.join("output.lowir");
            let mut command = Command::new(env!("CARGO_BIN_EXE_cppgm"));
            command
                .args([
                    "--emit-lowir",
                    "-g0",
                    &format!("-O{level}"),
                    "--stats",
                    "-o",
                ])
                .arg(&output)
                .arg(&reference)
                .arg(&definition);
            if validate {
                command.arg("--validate");
            }
            let result = command.output()?;
            assert!(result.status.success(), "{:?}", result.stderr);
            let text = std::fs::read_to_string(&output)?;
            assert!(!text.contains("declare global @shared"));
            assert_eq!(text.matches("global @shared :").count(), 1);
        }
    }
    Ok(())
}

#[test]
fn demanded_base_entries_keep_the_same_streamed_and_retained_view()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-base-observations-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    std::fs::write(&source, b"struct V { virtual int value(){return 1;} }; struct A: virtual V {}; A first; A second(first); struct D:A {}; D original; D copied(original); int main(){return second.value()+copied.value()-2;}\n")?;
    for extension in ["lowir", "view"] {
        for level in [0, 2] {
            let output = directory.0.join(format!("output.{extension}"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_cppgm"));
            command
                .args(["--emit-lowir", "-g0", &format!("-O{level}"), "-o"])
                .arg(&output)
                .arg(&source);
            let baseline = command.output()?;
            assert!(baseline.status.success(), "{:?}", baseline.stderr);
            let bytes = std::fs::read(&output)?;
            let validated = command.args(["--stats", "--validate"]).output()?;
            assert!(validated.status.success(), "{:?}", validated.stderr);
            assert_eq!(bytes, std::fs::read(&output)?);
            let text = std::str::from_utf8(&bytes)?;
            let mut names = std::collections::HashSet::new();
            for header in text
                .lines()
                .filter_map(|line| line.strip_prefix("function @"))
            {
                let name = header
                    .split_once('(')
                    .ok_or("missing function parameters")?
                    .0;
                assert!(names.insert(name), "duplicate definition: {name}");
            }
        }
    }
    Ok(())
}
