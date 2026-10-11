global @out = {
  i64 0
  i64 0
}

function @make_pair(%a : i64, %b : i64) -> obj<16x8> {
  slot $tmp : obj<16x8>
  block ^entry:
    %0 = addr $tmp
    store i64 %a, %0
    %1 = index i64 %0, 1
    store i64 %b, %1
    return obj<16x8> $tmp
}

function @main() -> i64 {
  block ^entry:
    %0 = call obj<16x8> @make_pair(4, 5)
    %1 = addr @out
    copyobj 16x8 %0, %1
    %2 = load i64 @out
    %3 = index i64 %1, 1
    %4 = load i64 %3
    %5 = binary add i64 %2, %4
    return i64 %5
}
