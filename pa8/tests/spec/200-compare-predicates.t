function @main() -> i64 {
  block ^entry:
    %0 = cmp ne i64 4, 5
    %1 = cmp le i64 4, 4
    %2 = cmp ge i64 5, 4
    %3 = binary add i64 %0, %1
    %4 = binary add i64 %3, %2
    return i64 %4
}
