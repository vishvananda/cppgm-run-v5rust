fn main() -> std::process::ExitCode {
    cppgm_rust::driver::lowiropt::run(&std::env::args_os().skip(1).collect::<Vec<_>>())
}
