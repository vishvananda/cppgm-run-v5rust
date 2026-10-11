function @main() -> i64 {
  block ^entry:
    %0 = const i64 5
    %1 = unary neg i64 %0
    %2 = cmp eq i64 %1, -5
    %3 = const i64 0
    %4 = unary not i64 %3
    %5 = cmp eq i64 %4, 1
    %6 = unary bitnot i64 %3
    %7 = cmp eq i64 %6, -1
    %8 = binary add i64 %2, %5
    %9 = binary add i64 %8, %7
    return i64 %9
}
