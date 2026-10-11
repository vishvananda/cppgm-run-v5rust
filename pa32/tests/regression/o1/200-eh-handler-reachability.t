declare function @may_throw() -> void
declare function @cleanup() -> void

function @main() -> void {
  block ^entry:
    eh_try ^dispatch
    call void @may_throw()
    eh_end
    return void

  block ^dispatch:
    call void @cleanup()
    resume
}
