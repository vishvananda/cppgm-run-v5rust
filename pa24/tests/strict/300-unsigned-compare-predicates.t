function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = cmp ult i64 1, -1
    %1 = cmp ule i64 5, 5
    %2 = cmp ugt i64 -1, 1
    %3 = cmp uge i64 5, 5
    %4 = binary add i64 %0, %1
    %5 = binary add i64 %4, %2
    %6 = binary add i64 %5, %3
    return i64 %6
}
