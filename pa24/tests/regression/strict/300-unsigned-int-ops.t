function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = binary udiv i64 -1, 2
    %1 = cmp eq i64 %0, 9223372036854775807
    %2 = binary umod i64 -1, 7
    %3 = cmp eq i64 %2, 1
    %4 = binary ushr i64 -16, 2
    %5 = cmp eq i64 %4, 4611686018427387900
    %6 = binary add i64 %1, %3
    %7 = binary add i64 %6, %5
    return i64 %7
}
