//! Unlinked batch transport takes raw request lines before ordinary CLI dispatch.
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn compiler_batch_precedes_help_and_preserves_request_records()
-> Result<(), Box<dyn std::error::Error>> {
    for arguments in [
        &["--batch-stdin"][..],
        &["--emit-ast", "--stats", "--help", "--batch-stdin"][..],
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cppgm"))
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or("missing request pipe")?
            .write_all(b"\xff\n\nlast\0")?;
        let result = child.wait_with_output()?;
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        assert_eq!(
            result.stdout,
            b"EXIT_NOT_IMPLEMENTED\nEXIT_NOT_IMPLEMENTED\nEXIT_NOT_IMPLEMENTED\n"
        );
    }
    Ok(())
}

#[test]
fn lowir_tool_stream_failures_retain_help_and_unlinked_batch_success()
-> Result<(), Box<dyn std::error::Error>> {
    for tool in [
        env!("CARGO_BIN_EXE_lowir"),
        env!("CARGO_BIN_EXE_lowiropt"),
        env!("CARGO_BIN_EXE_lowir2native"),
    ] {
        for arguments in [
            &["--help"][..],
            &["--batch-stdin", "--help", "--bad-option"][..],
        ] {
            let mut child = Command::new(tool)
                .args(arguments)
                .stdin(Stdio::piped())
                .stdout(std::fs::OpenOptions::new().write(true).open("/dev/full")?)
                .stderr(Stdio::piped())
                .spawn()?;
            let input = child.stdin.take().ok_or("missing request pipe")?;
            if arguments.contains(&"--batch-stdin") {
                let mut input = input;
                input.write_all(b"first\n\xff\n\nlast\0")?;
            }
            let output = child.wait_with_output()?;
            assert!(output.status.success(), "{tool}: {:?}", output.stderr);
            assert!(output.stderr.is_empty());
        }
        let output = Command::new(tool)
            .args(["--batch-stdin", "--help"])
            .stdin(std::fs::File::open("/")?)
            .output()?;
        assert!(output.status.success(), "{tool}: {:?}", output.stderr);
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        for arguments in [&[][..], &["--bad-option"][..]] {
            let output = Command::new(tool)
                .args(arguments)
                .stderr(std::fs::OpenOptions::new().write(true).open("/dev/full")?)
                .output()?;
            assert_eq!(output.status.code(), Some(1), "{tool}");
            assert!(output.stdout.is_empty());
        }
    }
    Ok(())
}

#[test]
fn failed_invocation_sinks_preserve_frontend_tool_statuses()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{fs::OpenOptions, process::Command};
    for (tool, arguments, stdout, status) in [
        (env!("CARGO_BIN_EXE_abimangle"), &["--help"][..], true, 0),
        (env!("CARGO_BIN_EXE_abimangle"), &[][..], false, 1),
        (env!("CARGO_BIN_EXE_preproc"), &[][..], false, 1),
        (env!("CARGO_BIN_EXE_preproc"), &["--help"][..], false, 1),
        (
            env!("CARGO_BIN_EXE_cppgm"),
            &["--emit-types", "--unsupported"][..],
            false,
            1,
        ),
    ] {
        let mut command = Command::new(tool);
        command.args(arguments);
        let sink = OpenOptions::new().write(true).open("/dev/full")?;
        if stdout {
            command.stdout(sink);
        } else {
            command.stderr(sink);
        }
        let result = command.output()?;
        assert_eq!(result.status.code(), Some(status), "{tool}: {arguments:?}");
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }
    Ok(())
}
