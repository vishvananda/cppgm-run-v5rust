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
