fn main() -> std::process::ExitCode {
    cppgm_rust::driver::lowir2native::run(
        &std::env::args_os().skip(1).collect::<Vec<_>>(),
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
