function @main() -> i64 {
  block ^entry:
    %0 = binary mul i64 6, 7
    %1 = cmp eq i64 %0, 42
    %2 = binary mod i64 43, 10
    %3 = cmp eq i64 %2, 3
    %4 = binary and i64 14, 11
    %5 = cmp eq i64 %4, 10
    %6 = binary or i64 8, 3
    %7 = cmp eq i64 %6, 11
    %8 = binary xor i64 14, 3
    %9 = cmp eq i64 %8, 13
    %10 = binary shl i64 3, 2
    %11 = cmp eq i64 %10, 12
    %12 = binary shr i64 -16, 2
    %13 = cmp eq i64 %12, -4
    %14 = binary add i64 %1, %3
    %15 = binary add i64 %14, %5
    %16 = binary add i64 %15, %7
    %17 = binary add i64 %16, %9
    %18 = binary add i64 %17, %11
    %19 = binary add i64 %18, %13
    return i64 %19
}
