function @extend() -> i128 {
  block ^entry:
    %value = convert sext i128 i128 9223372036854775808
    return i128 %value
}

function @zero_extend() -> i128 {
  block ^entry:
    %value = convert zext i128 i128 340282366920938463463374607431768211455
    return i128 %value
}

function @truncate() -> i128 {
  block ^entry:
    %value = convert trunc i128 i128 18446744073709551618
    return i128 %value
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %extended = call i128 @extend()
    %zero_extended = call i128 @zero_extend()
    %truncated = call i128 @truncate()
    %bad_extend = cmp ne i128 %extended, -9223372036854775808
    %bad_zero = cmp ne i128 %zero_extended, 18446744073709551615
    %bad_trunc = cmp ne i128 %truncated, 2
    %first = binary or i64 %bad_extend, %bad_zero
    %failed = binary or i64 %first, %bad_trunc
    return i64 %failed
}
