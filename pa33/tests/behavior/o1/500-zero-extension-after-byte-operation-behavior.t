global @wide_source = {
  i64 -255
}

global @byte_source = {
  i8 -127
  i8 0
}

function @flag_after_and(%p : ptr) -> i64 {
  block ^entry:
    %flag = load u8 %p
    %bit = binary and u8 %flag, 1
    %wide = convert zext i64 u8 %bit
    return i64 %wide
}

function @flag_after_or_xor(%p : ptr) -> i64 {
  block ^entry:
    %flag = load u8 %p
    %set = binary or u8 %flag, 6
    %flip = binary xor u8 %set, 4
    %wide = convert zext i64 u8 %flip
    return i64 %wide
}

function @sum_after_add(%p : ptr) -> i64 {
  block ^entry:
    %flag = load u8 %p
    %more = binary add u8 %flag, 200
    %wide = convert zext i64 u8 %more
    return i64 %wide
}

function @truncated_then_and(%p : ptr) -> i64 {
  block ^entry:
    %wide_value = load i64 %p
    %narrow = convert trunc u8 i64 %wide_value
    %bit = binary and u8 %narrow, 3
    %wide = convert zext i64 u8 %bit
    return i64 %wide
}

function @signed_after_and(%p : ptr) -> i64 {
  block ^entry:
    %value = load i8 %p
    %masked = binary and i8 %value, -2
    %wide = convert sext i64 i8 %masked
    return i64 %wide
}

function @mismatch(%actual : i64, %expected : i64) -> i64 {
  block ^entry:
    %bad = cmp ne i64 %actual, %expected
    %count = convert zext i64 i1 %bad
    return i64 %count
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %bytes = addr @byte_source
    %a = call i64 @flag_after_and(%bytes)
    %e1 = call i64 @mismatch(%a, 1)
    %b = call i64 @flag_after_or_xor(%bytes)
    %e2 = call i64 @mismatch(%b, 131)
    %c = call i64 @sum_after_add(%bytes)
    %e3 = call i64 @mismatch(%c, 73)
    %wide = addr @wide_source
    %d = call i64 @truncated_then_and(%wide)
    %e4 = call i64 @mismatch(%d, 1)
    %f = call i64 @signed_after_and(%bytes)
    %e5 = call i64 @mismatch(%f, -128)
    %s1 = binary add i64 %e1, %e2
    %s2 = binary add i64 %s1, %e3
    %s3 = binary add i64 %s2, %e4
    %s4 = binary add i64 %s3, %e5
    %exit = convert trunc i32 i64 %s4
    return i32 %exit
}
