function @main() -> i64 {
  block ^entry:
    %a = copy i64 4
    %b = binary add i64 %a, 3
    %c = binary mul i64 %b, 1
    return i64 %c
}
