mod source_tool;

fn main() -> std::process::ExitCode {
    source_tool::main(|source, output| cppgm_rust::driver::pptoken::emit_pptokens(source, output))
}
