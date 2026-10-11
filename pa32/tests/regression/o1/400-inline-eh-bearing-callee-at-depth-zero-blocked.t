declare function @poke(%value : i64) -> i64
declare function @trip(%value : i64) -> i64

global @bang_type : i8 = 0
global @snap_type : i8 = 0

function @guarded(%value : i64) -> i64 [prefer_local=yes] {
  block ^entry:
    eh_try ^catch_dispatch
    %result = call i64 @poke(%value)
    eh_end
    return i64 %result

  block ^catch_dispatch:
    eh_catch @bang_type, 1
    %selector = exception_selector i32
    %matched = cmp eq i32 %selector, 1
    branch %matched, ^catch_body, ^catch_next

  block ^catch_body:
    return i64 100

  block ^catch_next:
    resume
}

function @caller(%value : i64) -> i64 {
  block ^entry:
    %base = call i64 @guarded(%value)
    eh_try ^catch_dispatch
    %tail = call i64 @trip(%value)
    eh_end
    %result = binary add i64 %base, %tail
    return i64 %result

  block ^catch_dispatch:
    eh_catch @snap_type, 1
    %selector = exception_selector i32
    %matched = cmp eq i32 %selector, 1
    branch %matched, ^catch_body, ^catch_next

  block ^catch_body:
    return i64 200

  block ^catch_next:
    resume
}
