declare function @__builtin_strlen(%arg0 : ptr) -> i64 [effects=readonly, unwind=no, binding=strong, object=cppgm_builtin_strlen]
declare function @__builtin_memcmp(%arg0 : ptr, %arg1 : ptr, %arg2 : i64) -> i32 [effects=readonly, unwind=no, binding=strong, object=cppgm_builtin_memcmp]
declare function @__cxa_allocate_exception(%arg0 : i64) -> ptr [role=eh_allocate_exception, linkage=c, binding=strong, object=__cxa_allocate_exception]
declare function @__cxa_begin_catch(%arg0 : ptr) -> ptr [role=eh_begin_catch, linkage=c, binding=strong, object=__cxa_begin_catch]
declare function @__cxa_throw(%arg0 : ptr, %arg1 : ptr, %arg2 : ptr) -> void [return=noreturn, role=eh_throw, linkage=c, binding=strong, object=__cxa_throw]
declare function @std_terminate() -> void [unwind=no, return=noreturn, role=terminate, binding=strong, object=_ZSt9terminatev]
declare function @observe(%value : i64) -> void [unwind=no]
declare global @out_of_range_rtti [binding=strong, object=_ZTISt12out_of_range]

global @abc [storage=readonly, binding=internal] = {
  i8 97
  i8 98
  i8 99
  i8 0
}

function @call_terminate(%exception_object : ptr) -> void [unwind=no, return=noreturn, binding=internal, no_inline=yes] {
  block ^entry:
    %t1 = call ptr @__cxa_begin_catch(%exception_object)
    call void @std_terminate()
    return void
}

function @equals_text(%text : ptr, %size : i64, %rhs : ptr) -> u8 [unwind=no, binding=weak, inline_hint=yes] {
  block ^entry:
    eh_try ^landing
    %len = call i64 @__builtin_strlen(%rhs)
    %same_size = cmp eq i64 %len, %size
    branch %same_size, ^check, ^differ

  block ^differ:
    eh_end
    return u8 0

  block ^check:
    %npos = cmp eq i64 %len, -1
    branch %npos, ^throw, ^compare

  block ^throw:
    %exception = call ptr @__cxa_allocate_exception(16)
    %rtti = addr @out_of_range_rtti
    call void @__cxa_throw(%exception, %rtti, %rtti)
    return u8 0

  block ^compare:
    %order = call i32 @__builtin_memcmp(%text, %rhs, %len)
    %equal = cmp eq i32 %order, 0
    %result = convert trunc u8 i64 %equal
    eh_end
    return u8 %result

  block ^landing:
    eh_catch_all, 1
    %caught = exception ptr
    call void @call_terminate(%caught)
    return u8 0
}

function @at_most_four(%base : ptr, %index : i64) -> i64 [unwind=no, binding=weak] {
  block ^entry:
    eh_try ^landing
    %outside = cmp uge i64 %index, 4
    branch %outside, ^throw, ^load

  block ^throw:
    %exception = call ptr @__cxa_allocate_exception(16)
    %rtti = addr @out_of_range_rtti
    call void @__cxa_throw(%exception, %rtti, %rtti)
    return i64 0

  block ^load:
    %slot = index i64 %base, %index
    %value = load i64 %slot
    eh_end
    return i64 %value

  block ^landing:
    eh_catch_all, 1
    %caught = exception ptr
    call void @call_terminate(%caught)
    return i64 0
}

function @is_abc(%text : ptr, %size : i64) -> u8 [binding=strong] {
  block ^entry:
    %literal = addr @abc
    %result = call u8 @equals_text(%text, %size, %literal)
    return u8 %result
}

function @is_other(%text : ptr, %size : i64, %rhs : ptr) -> u8 [binding=strong] {
  block ^entry:
    %result = call u8 @equals_text(%text, %size, %rhs)
    return u8 %result
}

function @third(%base : ptr) -> i64 [binding=strong] {
  block ^entry:
    %value = call i64 @at_most_four(%base, 2)
    return i64 %value
}

function @nth(%base : ptr, %n : i64) -> i64 [binding=strong] {
  block ^entry:
    %value = call i64 @at_most_four(%base, %n)
    return i64 %value
}
