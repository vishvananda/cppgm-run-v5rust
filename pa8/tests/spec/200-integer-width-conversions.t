function @main() -> i64 {
  block ^entry:
    %0 = convert zext i64 u8 255
    %1 = convert sext i64 i8 -1
    %2 = convert trunc i32 i64 65537
    %3 = cmp eq i64 %0, 255
    %4 = cmp eq i64 %1, -1
    %5 = convert sext i64 i32 %2
    %6 = cmp eq i64 %5, 1
    %7 = binary add i64 %3, %4
    %8 = binary add i64 %7, %6
    return i64 %8
}
