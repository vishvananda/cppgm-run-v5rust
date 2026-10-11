declare function @may_throw() -> void
declare function @end_catch() -> void [unwind=no]

function @main() -> void {
  block ^entry:
    eh_try ^catch_dispatch
    call void @may_throw()
    eh_end
    return void

  block ^catch_dispatch:
    eh_catch_all
    %ex = exception ptr
    eh_cleanup ^catch_cleanup
    branch 0, ^rethrow, ^catch_finish

  block ^rethrow:
    resume

  block ^catch_finish:
    eh_end
    call void @end_catch()
    return void

  block ^catch_cleanup:
    call void @end_catch()
    eh_end
    resume
}
