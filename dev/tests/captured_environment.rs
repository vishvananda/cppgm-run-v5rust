//! Build-selected configuration remains available without runtime host tools.
#[path = "../bin/builtin_host_configuration.rs"]
pub mod captured;
use std::process::Command;

fn compiler() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cppgm"));
    command.env("PATH", "/nonexistent");
    command
}

#[test]
fn direct_queries_use_captured_bytes_without_a_host() -> Result<(), Box<dyn std::error::Error>> {
    let banner = [
        b"cppgm++ (host configuration: ".as_slice(),
        captured::COMMAND,
        b")\n",
    ]
    .concat();
    for (flag, expected) in [
        ("--version", banner.as_slice()),
        ("-v", banner.as_slice()),
        ("-dumpmachine", captured::TARGET),
        ("-dumpversion", captured::VERSION),
        ("-print-search-dirs", captured::SEARCH),
    ] {
        let output = compiler().arg(flag).output()?;
        assert!(output.status.success(), "{flag}: {:?}", output.stderr);
        assert_eq!(output.stdout, expected, "{flag}");
        assert!(output.stderr.is_empty());
    }
    for args in [["--version", "input.cpp"], ["-c", "-dumpmachine"]] {
        let output = compiler().args(args).output()?;
        assert!(!output.status.success());
        assert_eq!(
            output.stderr,
            b"ERROR: query flag must be used as a direct invocation\n"
        );
    }
    Ok(())
}

#[test]
fn query_output_failure_has_no_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    for flag in [
        "--version",
        "-v",
        "-dumpmachine",
        "-dumpversion",
        "-print-search-dirs",
    ] {
        let output = compiler()
            .arg(flag)
            .stdout(std::fs::OpenOptions::new().write(true).open("/dev/full")?)
            .output()?;
        assert_eq!(output.status.code(), Some(1), "{flag}");
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty(), "{flag}: {:?}", output.stderr);
    }
    Ok(())
}

#[test]
fn source_frontend_uses_captured_macros_without_a_host() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("cppgm-host-capture-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let source = directory.join("unit.cpp");
    let object = directory.join("unit.o");
    std::fs::write(&source, "int value() { return __GNUC__; }\n")?;
    let preprocessed = compiler().arg("-E").arg(&source).output()?;
    assert!(preprocessed.status.success(), "{:?}", preprocessed.stderr);
    assert!(
        !preprocessed
            .stdout
            .windows(b"__GNUC__".len())
            .any(|w| w == b"__GNUC__")
    );
    let output = compiler()
        .args(["-c", "-g0", "-o"])
        .arg(&object)
        .arg(&source)
        .output()?;
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(std::fs::read(&object)?.starts_with(b"\x7fELF"));
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
