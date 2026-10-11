global @dst = {
  i64 7
  i64 8
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = addr @dst
    zeroinit 16x8 %0
    %1 = load i64 @dst
    %2 = index i64 %0, 1
    %3 = load i64 %2
    %4 = binary add i64 %1, %3
    return i64 %4
}
