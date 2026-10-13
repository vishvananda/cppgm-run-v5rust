fn main() -> std::process::ExitCode {
    cppgm_rust::driver::lowir::run(&std::env::args().skip(1).collect::<Vec<_>>())
}
