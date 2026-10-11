function @main() -> i64 {
  slot $x : i64
  slot $p : ptr

  block ^entry:
    %0 = addr $x
    store ptr %0, $p
    %1 = load ptr $p
    %2 = const i64 0
    %3 = index i64 %1, %2
    %4 = const i64 7
    store i64 %4, %3
    %5 = load i64 $x
    return i64 %5
}
