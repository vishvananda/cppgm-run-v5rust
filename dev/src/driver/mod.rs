//! Command-line boundary shared by the nine compiler tools.

/// Return the distinct scaffold status until this tool is implemented.
#[must_use]
pub fn run_tool(tool: &str) -> std::process::ExitCode {
    eprintln!("{tool}: Rust port is not implemented yet");
    std::process::ExitCode::from(86)
}
