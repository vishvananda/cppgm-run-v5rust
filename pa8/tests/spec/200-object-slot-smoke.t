function @main() -> i64 {
  slot $tmp : obj<16x8>
  block ^entry:
    %0 = addr $tmp
    zeroinit 16x8 %0
    %1 = index i64 %0, 1
    store i64 7, %1
    %2 = load i64 %1
    return i64 %2
}
