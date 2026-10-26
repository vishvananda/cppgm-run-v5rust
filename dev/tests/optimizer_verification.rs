//! Driver audit options reach the shared optimizer without changing object bytes.
use std::{ffi::OsString, path::PathBuf, process::Command};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn compile(arguments: &[OsString]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_cppgm"))
        .args(arguments)
        .output()?;
    assert!(
        output.status.success(),
        "{arguments:?}: {:?}",
        output.stderr
    );
    assert!(output.stdout.is_empty());
    assert!(
        output.stderr.is_empty(),
        "{arguments:?}: {:?}",
        output.stderr
    );
    Ok(())
}
#[test]
fn verification_reaches_source_and_serialized_lowir_objects_without_changing_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = Scratch(
        std::env::temp_dir().join(format!("cppgm-pass-verification-{}", std::process::id())),
    );
    std::fs::create_dir_all(&directory.0)?;
    let source = directory.0.join("input.cpp");
    std::fs::write(
        &source,
        b"template<class T> T step(T value) { return value + 1; }\n\
        extern int read(int);\n\
        int sum(int n) { int value = 0; for (int i = 0; i < n; ++i) {\n\
        try { value += read(step(i)); } catch (...) { value -= i; } } return value; }\n",
    )?;
    let serialized = directory.0.join("input.lowir");
    compile(&[
        "--emit-lowir".into(),
        "-g0".into(),
        "-O0".into(),
        "-o".into(),
        serialized.clone().into(),
        source.clone().into(),
    ])?;
    for input in [source, serialized.clone()] {
        for level in 0..=3 {
            for extension in ["o", "obj"] {
                let output = directory.0.join(format!("output.{extension}"));
                let arguments = vec![
                    "-c".into(),
                    format!("-O{level}").into(),
                    "-o".into(),
                    output.clone().into(),
                    input.clone().into(),
                ];
                compile(&arguments)?;
                let baseline = std::fs::read(&output)?;
                let mut verified = arguments;
                let dump = directory.0.join("unused.dump");
                let mut option = OsString::from("--verify-passes=");
                option.push(dump.as_os_str());
                verified.push(option);
                compile(&verified)?;
                assert_eq!(baseline, std::fs::read(&output)?);
                assert!(!dump.exists());
                verified.push("--stats-functions".into());
                verified.push("--stats".into());
                let observed = Command::new(env!("CARGO_BIN_EXE_cppgm"))
                    .args(&verified)
                    .output()?;
                assert!(observed.status.success(), "{:?}", observed.stderr);
                assert!(observed.stdout.is_empty());
                let census = std::str::from_utf8(&observed.stderr)?;
                assert_eq!(
                    census.starts_with("cppgm++ compile stats "),
                    input != serialized,
                    "{census}"
                );
                assert!(
                    census
                        .lines()
                        .any(|line| line.starts_with("function_census "))
                );
                if level == 0 {
                    let label = if input == serialized {
                        "header_label=for_cond_"
                    } else {
                        "header_label=block_"
                    };
                    assert!(census.contains(label), "{census}");
                }
                assert_eq!(baseline, std::fs::read(&output)?);
            }
        }
    }
    Ok(())
}
