declare function @exit(%status : i32) -> void [unwind=no]

function @max(%a : ptr, %b : ptr) -> ptr [binding=internal, inline_hint=yes, unwind=no] {
  block ^entry:
    %x = load i64 %a
    %y = load i64 %b
    %less = cmp ult i64 %x, %y
    branch %less, ^then, ^else

  block ^then:
    return ptr %b

  block ^else:
    return ptr %a
}

function @grow(%begin : ptr, %end : ptr, %n : i64) -> i64 [binding=internal, unwind=no] {
  slot $size : i64
  slot $n : i64

  block ^entry:
    %size = binary sub ptr %end, %begin
    store i64 %size, $size
    store i64 %n, $n
    %a = addr $size
    %b = addr $n
    %m = call ptr @max(%a, %b)
    %v = load i64 %m
    %r = binary add i64 %size, %v
    return i64 %r
}

function @main() -> i32 [role=entry, unwind=no] {
  slot $buffer : obj<16x1>

  block ^entry:
    %p = addr $buffer
    %q = index i8 %p, 5
    %first = call i64 @grow(%p, %q, 3)
    %second = call i64 @grow(%p, %q, 9)
    %bad_first = cmp ne i64 %first, 10
    %bad_second = cmp ne i64 %second, 14
    %first_bad = convert zext i64 i1 %bad_first
    %second_bad = convert zext i64 i1 %bad_second
    %bad = binary or i64 %first_bad, %second_bad
    %status = convert trunc i32 i64 %bad
    return i32 %status
}
