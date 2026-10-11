mod source_tool;

fn main() -> std::process::ExitCode {
    source_tool::main(|source, output| cppgm_rust::driver::ppexpr::emit_expressions(source, output))
}
