function @main() -> i64 {
  slot $x : i64

  block ^entry:
    %0 = const i64 1
    %1 = const i64 2
    %2 = binary add i64 %0, %1
    store i64 %2, $x
    %3 = load i64 $x
    %4 = const i64 3
    %5 = binary add i64 %3, %4
    return i64 %5
}
