function @main(%x : i64, %y : i64) -> i64 {
  block ^entry:
    %a = binary add i64 %x, %y
    %b = binary add i64 %x, %y
    %c = binary mul i64 %a, %b
    return i64 %c
}
