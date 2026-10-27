//! Source reports observe a parse without changing the source view or flag ownership.
use std::{collections::BTreeMap, path::PathBuf, process::Command};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _status = std::fs::remove_dir_all(&self.0);
    }
}
fn counters(line: &str) -> BTreeMap<&str, &str> {
    line.split_whitespace()
        .filter_map(|field| field.split_once('='))
        .filter(|(key, _)| !key.ends_with("_ms") && *key != "peak_rss_kb")
        .collect()
}
#[test]
fn explicit_this_observes_the_same_member_work_as_implicit_this()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-this-observations-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    let output = directory.0.join("view");
    let mut previous = None;
    for member in ["value", "this->value"] {
        std::fs::write(
            &source,
            format!("struct X{{int value; int read()const{{return {member};}}}};\n"),
        )?;
        let result = Command::new(env!("CARGO_BIN_EXE_cppgm"))
            .args(["--emit-semantics", "--stats", "-o"])
            .arg(&output)
            .arg(&source)
            .output()?;
        assert!(result.status.success(), "{:?}", result.stderr);
        let stderr = String::from_utf8(result.stderr)?;
        let line = stderr
            .lines()
            .find(|line| line.starts_with("cppgm++ semantics "))
            .ok_or("missing semantic observations")?;
        let expressions = counters(line)
            .get("expressions")
            .ok_or("missing expression observations")?
            .parse::<u64>()?;
        if let Some(implicit) = previous {
            assert_eq!(expressions, implicit);
        }
        previous = Some(expressions);
    }
    Ok(())
}
#[test]
fn reports_retain_parse_work_and_follow_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-source-observations-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    std::fs::write(
        &source,
        b"const char* author = __CPPGM_AUTHOR__;\n\
          struct Counter { int step(int n) { return n + 1; } };\n",
    )?;
    let output = directory.0.join("output.ast");
    let mut command = Command::new(env!("CARGO_BIN_EXE_cppgm"));
    command.args(["--emit-ast", "-o"]).arg(&output).arg(&source);
    let plain = command.output()?;
    assert!(plain.status.success(), "{:?}", plain.stderr);
    assert!(plain.stderr.is_empty());
    let bytes = std::fs::read(&output)?;
    assert!(
        bytes
            .windows(b"Vish Abrams".len())
            .any(|window| window == b"Vish Abrams")
    );
    command.arg("--stats");
    let mut previous = None;
    for _ in 0..2 {
        let reported = command.output()?;
        assert!(reported.status.success(), "{:?}", reported.stderr);
        assert!(reported.stdout.is_empty());
        assert_eq!(bytes, std::fs::read(&output)?);
        let stderr = String::from_utf8(reported.stderr)?;
        assert_eq!(stderr.lines().count(), 1);
        assert!(stderr.starts_with("cppgm++ stats "));
        let fields = counters(&stderr)
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect::<BTreeMap<_, _>>();
        for field in [
            "tokens",
            "checkpoints",
            "deferred_bodies",
            "lookups",
            "spelling_bytes",
        ] {
            assert!(
                fields
                    .get(field)
                    .ok_or("missing source observation")?
                    .parse::<u64>()?
                    > 0
            );
        }
        if let Some(previous) = &previous {
            assert_eq!(previous, &fields);
        }
        previous = Some(fields);
    }
    command.arg("--stats");
    let repeated = command.output()?;
    assert_eq!(repeated.status.code(), Some(1));
    assert_eq!(
        repeated.stderr,
        b"ERROR: unsupported option in emit mode: --stats\n"
    );
    let invalid = directory.0.join("invalid.cpp");
    std::fs::write(&invalid, b"int f( {\n")?;
    let rejected = Command::new(env!("CARGO_BIN_EXE_cppgm"))
        .args(["--emit-ast", "--stats", "-o"])
        .arg(&output)
        .arg(&invalid)
        .output()?;
    assert_eq!(rejected.status.code(), Some(1));
    let stderr = String::from_utf8(rejected.stderr)?;
    let mut lines = stderr.lines();
    assert!(
        lines
            .next()
            .ok_or("missing diagnostic")?
            .starts_with("ERROR: ")
    );
    assert!(
        lines
            .next()
            .ok_or("missing failure report")?
            .starts_with("cppgm++ stats ")
    );
    assert!(lines.next().is_none());
    Ok(())
}

#[test]
fn emit_invocation_preserves_level_spelling_and_argument_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let directory =
        Scratch(std::env::temp_dir().join(format!("cppgm-emit-invocation-{}", std::process::id())));
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    std::fs::write(
        &source,
        b"#ifndef __OPTIMIZE__\n#error optimization macro missing\n#endif\n\
          int inc(int n){return n+1;}int main(){return inc(0)-1;}\n",
    )?;
    let output = directory.0.join("output.lowir");
    let emit = |mode: &str, flags: &[OsString]| {
        Command::new(env!("CARGO_BIN_EXE_cppgm"))
            .args([mode, "-o"])
            .arg(&output)
            .arg(&source)
            .args(flags)
            .output()
    };
    for (alias, level) in [("-O", "-O1"), ("-O00", "-O3")] {
        let baseline = emit("--emit-lowir", &[level.into()])?;
        assert!(baseline.status.success(), "{:?}", baseline.stderr);
        let bytes = std::fs::read(&output)?;
        let aliased = emit("--emit-lowir", &[alias.into()])?;
        assert!(aliased.status.success(), "{:?}", aliased.stderr);
        assert_eq!(bytes, std::fs::read(&output)?);
    }
    let ignored = directory.0.join("ignored");
    let bytes = std::fs::read(&output)?;
    for flag in ["--witness", "--witness-debug"] {
        let result = emit(
            "--emit-lowir",
            &["-O3".into(), flag.into(), ignored.clone().into_os_string()],
        )?;
        assert!(result.status.success(), "{:?}", result.stderr);
        assert_eq!(bytes, std::fs::read(&output)?);
        assert!(!ignored.exists());
    }
    for mode in [
        "--emit-ast",
        "--emit-types",
        "--emit-semantics",
        "--emit-lowir",
    ] {
        let rejected = emit(mode, &["-E".into()])?;
        assert_eq!(rejected.status.code(), Some(1));
        assert_eq!(rejected.stderr, b"ERROR: invalid usage\n");
        let rejected = emit(mode, &[OsString::from_vec(b"--bad-\xff".to_vec())])?;
        assert_eq!(rejected.status.code(), Some(1));
        assert_eq!(
            rejected.stderr,
            b"ERROR: unsupported option in emit mode: --bad-\xff\n"
        );
        assert_eq!(bytes, std::fs::read(&output)?);
    }
    Ok(())
}

#[test]
fn opened_source_read_failures_remain_unlocated_and_skip_statistics()
-> Result<(), Box<dyn std::error::Error>> {
    let directory =
        Scratch(std::env::temp_dir().join(format!("cppgm-source-read-{}", std::process::id())));
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("include.cpp");
    std::fs::write(&source, b"int before;\n#include \"/proc/self/mem\"\n")?;
    let primary = std::path::Path::new("/proc/self/mem");
    let expected = b"ERROR: basic_filebuf::xsgetn error reading the file: Input/output error\n";
    for path in [primary, &source] {
        for mode in [
            "--emit-ast",
            "--emit-types",
            "--emit-semantics",
            "--emit-lowir",
        ] {
            let output = directory.0.join(mode);
            let result = Command::new(env!("CARGO_BIN_EXE_cppgm"))
                .args([mode, "-o"])
                .arg(&output)
                .arg(path)
                .output()?;
            assert_eq!(result.status.code(), Some(1));
            assert_eq!(result.stderr, expected);
            if mode == "--emit-lowir" {
                assert!(!output.exists());
            } else {
                assert_eq!(std::fs::read(&output)?, b"1 translation units\n");
                std::fs::remove_file(&output)?;
            }
        }
        let result = Command::new(env!("CARGO_BIN_EXE_cppgm"))
            .args(["--emit-ast", "--stats", "-o"])
            .arg(directory.0.join("reported.ast"))
            .arg(path)
            .output()?;
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(result.stderr, expected);
        let result = Command::new(env!("CARGO_BIN_EXE_preproc"))
            .args(["--stats", "-o"])
            .arg(directory.0.join("tokens"))
            .arg(path)
            .output()?;
        assert_eq!(result.status.code(), Some(1));
        assert_eq!(result.stderr, expected);
    }
    Ok(())
}

#[test]
fn semantic_reports_follow_success_and_do_not_change_the_view()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-semantic-reports-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    let output = directory.0.join("view");
    std::fs::write(
        &source,
        b"void target(); int main(){target(); return 1+2;}\n",
    )?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_cppgm"));
    command
        .args(["--emit-semantics", "-o"])
        .arg(&output)
        .arg(&source);
    let plain = command.output()?;
    assert!(plain.status.success());
    assert!(plain.stderr.is_empty());
    let bytes = std::fs::read(&output)?;
    command.arg("--stats");
    let reported = command.output()?;
    assert!(reported.status.success());
    assert!(reported.stdout.is_empty());
    assert_eq!(std::fs::read(&output)?, bytes);
    let stderr = String::from_utf8(reported.stderr)?;
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("cppgm++ stats "));
    assert!(lines[1].starts_with("cppgm++ semantics "));
    let work = counters(lines[1]);
    for field in ["expressions", "conversions", "demands"] {
        assert!(
            work.get(field)
                .ok_or("missing semantic observation")?
                .parse::<u64>()?
                > 0
        );
    }
    for invalid in [
        b"int main(){return ;\n".as_slice(),
        b"int main(){return missing;}\n",
    ] {
        std::fs::write(&source, invalid)?;
        let rejected = command.output()?;
        assert_eq!(rejected.status.code(), Some(1));
        let stderr = String::from_utf8(rejected.stderr)?;
        assert!(stderr.starts_with("ERROR: "));
        assert!(!stderr.contains("cppgm++ stats "));
        assert!(!stderr.contains("cppgm++ semantics "));
    }
    command.arg("--stats");
    let repeated = command.output()?;
    assert_eq!(repeated.status.code(), Some(1));
    assert_eq!(
        repeated.stderr,
        b"ERROR: unsupported option in emit mode: --stats\n"
    );
    Ok(())
}
