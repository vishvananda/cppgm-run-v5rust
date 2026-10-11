function @main() -> i64 [role=entry] {
  slot $signed : i32
  slot $unsigned : u32
  block ^entry:
    store i32 -3, $signed
    store u32 4294967295, $unsigned
    %raw = load i32 $signed
    %wide = convert sext i64 i32 %raw
    %changed = binary add i64 %wide, 7
    %original_bad = cmp ne i32 %raw, -3
    %result_bad = cmp ne i64 %changed, 4
    %zero_signed = convert zext i64 i32 %raw
    %zero_bad = cmp ne i64 %zero_signed, 4294967293
    %unsigned = load u32 $unsigned
    %extended = convert zext i64 u32 %unsigned
    %unsigned_bad = cmp ne i64 %extended, 4294967295
    %sign_unsigned = convert sext i64 u32 %unsigned
    %sign_bad = cmp ne i64 %sign_unsigned, -1
    %bad = binary or i64 %original_bad, %result_bad
    %bad2 = binary or i64 %bad, %unsigned_bad
    %bad3 = binary or i64 %bad2, %zero_bad
    %result = binary or i64 %bad3, %sign_bad
    return i64 %result
}
