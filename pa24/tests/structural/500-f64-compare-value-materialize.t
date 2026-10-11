function @main() -> i64 [role=entry] {
  block ^entry:
    %a = cmp eq f64 1.5, 1.5
    %b = binary add i64 %a, %a
    return i64 %b
}
