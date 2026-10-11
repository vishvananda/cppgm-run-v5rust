function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = convert fptosi i64 f64 -42.75
    %1 = cmp eq i64 %0, -42
    %2 = convert fptoui i64 f64 42.75
    %3 = cmp eq i64 %2, 42
    %4 = convert sitofp f80 i64 -42
    %5 = convert fptosi i64 f80 %4
    %6 = cmp eq i64 %5, -42
    %7 = convert uitofp f80 i64 -1
    %8 = convert fptoui i64 f80 %7
    %9 = cmp eq i64 %8, -1
    %10 = binary add i64 %1, %3
    %11 = binary add i64 %10, %6
    %12 = binary add i64 %11, %9
    return i64 %12
}
