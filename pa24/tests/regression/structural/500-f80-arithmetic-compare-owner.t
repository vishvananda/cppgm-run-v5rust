function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const f80 1.25L
    %b = const f80 2.5L
    %sum = binary add f80 %a, %b
    %ok = cmp eq f80 %sum, 3.75L
    return i64 %ok
}
