function @low_word(%value : i128) -> i64 {
  block ^entry:
    return i64 %value
}

function @float_bits(%value : f64) -> i64 {
  block ^entry:
    return i64 %value
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %low = call i64 @low_word(18446744073709551623)
    %bad_low = cmp ne i64 %low, 7
    %bits = call i64 @float_bits(42.0)
    %bad_bits = cmp ne i64 %bits, 4631107791820423168
    %failed = binary or i64 %bad_low, %bad_bits
    return i64 %failed
}
