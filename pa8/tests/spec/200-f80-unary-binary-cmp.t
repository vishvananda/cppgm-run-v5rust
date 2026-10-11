function @main() -> i64 {
  block ^entry:
    %0 = const f80 2.5L
    %1 = unary neg f80 %0
    %2 = const f80 -2.5L
    %3 = cmp eq f80 %1, %2
    %4 = const f80 8.0L
    %5 = const f80 2.0L
    %6 = binary div f80 %4, %5
    %7 = const f80 4.0L
    %8 = cmp eq f80 %6, %7
    %9 = binary add i64 %3, %8
    return i64 %9
}
