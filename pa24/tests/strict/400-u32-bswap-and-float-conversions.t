function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = unary bswap u32 305419896
    %1 = cmp eq u32 %0, 2018915346
    %2 = convert uitofp f64 u32 %0
    %3 = convert fptoui u32 f64 %2
    %4 = cmp eq u32 %3, 2018915346
    %5 = binary add i64 %1, %4
    %6 = cmp eq i64 %5, 2
    %7 = cmp eq i64 %6, 0
    return i64 %7
}
