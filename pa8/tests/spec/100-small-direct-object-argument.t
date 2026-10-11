function @sum(%bias : i64, %value : obj<8x4>) -> i64 {
  block ^entry:
    %tail = index i32 %value, 1
    %first = load i32 %value
    %second = load i32 %tail
    %a = convert sext i64 i32 %first
    %b = convert sext i64 i32 %second
    %pair = binary add i64 %a, %b
    %result = binary add i64 %bias, %pair
    return i64 %result
}

function @main() -> i64 {
  slot $value : obj<8x4>
  block ^entry:
    %address = addr $value
    store i32 4, %address
    %tail = index i32 %address, 1
    store i32 5, %tail
    %result = call i64 @sum(1, $value)
    return i64 %result
}
