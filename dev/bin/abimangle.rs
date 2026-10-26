#[path = "source_tool/stack_limit.rs"]
mod stack_limit;

fn main() -> std::process::ExitCode {
    let Ok(budget) = cppgm_rust::support::stack::StackBudget::from_limit(stack_limit::query())
    else {
        use std::io::Write;
        let _ = std::io::stderr()
            .lock()
            .write_all(b"abimangle: host stack budget exceeds usize\n");
        return std::process::ExitCode::FAILURE;
    };
    cppgm_rust::driver::abimangle::run(
        &std::env::args_os().skip(1).collect::<Vec<_>>(),
        budget,
        peak_rss,
    )
}

fn peak_rss() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")?
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}
