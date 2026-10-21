global @value : i64 = 7

function @truncate(%value : i64) -> i64 {
  block ^entry:
    %result = convert trunc i64 i128 %value
    return i64 %result
}

function @main() -> i64 [role=entry] {
  slot $wide : i128
  block ^entry:
    store volatile i128 18446744073709551657, $wide
    %stored = convert trunc i64 i128 $wide
    %literal = convert trunc i64 i128 18446744073709551657
    %symbol = convert trunc i64 i128 @value
    %register = call i64 @truncate(41)
    %bad_stored = cmp ne i64 %stored, 41
    %bad_literal = cmp ne i64 %literal, 41
    %bad_symbol = cmp ne i64 %symbol, 0
    %bad_register = cmp ne i64 %register, 0
    %a = binary or i64 %bad_stored, %bad_literal
    %b = binary or i64 %bad_symbol, %bad_register
    %bad = binary or i64 %a, %b
    return i64 %bad
}
