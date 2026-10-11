global @src = {
  i64 5
  i64 9
}

global @dst = {
  i64 0
  i64 0
}

function @main() -> i64 {
  block ^entry:
    %0 = addr @dst
    %1 = addr @src
    copyobj 16x8 %1, %0
    %2 = load i64 @dst
    %3 = index i64 %0, 1
    %4 = load i64 %3
    %5 = binary add i64 %2, %4
    return i64 %5
}
