function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const i64 1
    %b = const i64 2
    %c = binary add i64 %a, %b
    %d = binary mul i64 %c, 4
    %e = binary sub i64 %d, 12
    return i64 %e
}
