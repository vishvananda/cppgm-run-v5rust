#[path = "source_tool/stack_limit.rs"]
mod stack_limit;

fn main() -> std::process::ExitCode {
    let Ok(budget) = cppgm_rust::support::stack::StackBudget::from_limit(stack_limit::query())
    else {
        eprintln!("abimangle: host stack budget exceeds usize");
        return std::process::ExitCode::FAILURE;
    };
    cppgm_rust::driver::abimangle::run(&std::env::args_os().skip(1).collect::<Vec<_>>(), budget)
}
