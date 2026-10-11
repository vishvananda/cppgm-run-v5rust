function @main() -> i64 {
  block ^entry:
    %0 = const i16 4660
    %1 = unary bswap i16 %0
    %2 = cmp eq i16 %1, 13330

    %3 = const i32 287454020
    %4 = unary bswap i32 %3
    %5 = cmp eq i32 %4, 1144201745

    %6 = const i64 72623859790382856
    %7 = unary bswap i64 %6
    %8 = cmp eq i64 %7, 578437695752307201

    %9 = binary add i64 %2, %5
    %10 = binary add i64 %9, %8
    return i64 %10
}
