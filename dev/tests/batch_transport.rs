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
