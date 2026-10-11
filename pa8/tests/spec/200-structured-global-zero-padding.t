global @blob = {
  i64 7
  zero 8
  i64 11
}

function @main() -> i64 {
  block ^entry:
    %0 = addr @blob
    %1 = index i64 %0, 2
    %2 = load i64 @blob
    %3 = load i64 %1
    %4 = binary add i64 %2, %3
    return i64 %4
}
