declare function @poke(%value : i64) -> i64

global @bang_type : i8 = 0

function @update_flag(%this : ptr, %size : i64) -> u8 {
  block ^entry:
    eh_try ^catch_dispatch
    %checked = call i64 @poke(%size)
    eh_end
    %p = copy ptr %this
    %narrow = convert trunc u8 i64 %checked
    %shifted = binary shl u8 %narrow, 1
    %old = load u8 %p
    %low = binary and u8 %old, 1
    %merged = binary or u8 %low, %shifted
    store u8 %merged, %p
    %q = copy ptr %this
    %again = load u8 %q
    %cleared = binary and u8 %again, 254
    store u8 %cleared, %q
    return u8 %cleared

  block ^catch_dispatch:
    eh_catch @bang_type, 1
    %selector = exception_selector i32
    %matched = cmp eq i32 %selector, 1
    branch %matched, ^catch_body, ^catch_next

  block ^catch_body:
    return u8 0

  block ^catch_next:
    resume
}

function @update_flag_kept_across_call(%this : ptr, %size : i64) -> u8 {
  block ^entry:
    %p = copy ptr %this
    %narrow = convert trunc u8 i64 %size
    store u8 %narrow, %p
    %checked = call i64 @poke(%size)
    %q = copy ptr %this
    %again = load u8 %q
    %tail = convert trunc u8 i64 %checked
    %sum = binary add u8 %again, %tail
    return u8 %sum
}
