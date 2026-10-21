global @value : obj<32x8> = 0x0102030405060708090a0b0c0d0e0f10

function @main() -> i64 [role=entry] {
  block ^entry:
    %low = load i64 @value
    %high_address = index i8 @value, 8
    %high = load i64 %high_address
    %repeat_address = index i8 @value, 24
    %repeat = load i64 %repeat_address
    %bad_low = cmp ne i64 %low, 651345242494996240
    %bad_high = cmp ne i64 %high, 0
    %bad_repeat = cmp ne i64 %repeat, %high
    %bad_a = binary or i64 %bad_low, %bad_high
    %failed = binary or i64 %bad_a, %bad_repeat
    return i64 %failed
}
