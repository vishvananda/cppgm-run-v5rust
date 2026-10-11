function @sum_object_and_scalars(%pair : obj<16x8>, %a : i64, %b : i64, %c : i64, %d : i64, %tail : i64) -> i64 {
  slot $pair : obj<16x8>

  block ^entry:
    %address = addr $pair
    copyobj 16x8 %pair, %address
    %low = load i64 %address
    %high_address = index i8 %address, 8
    %high = load i64 %high_address
    %sum1 = binary add i64 %low, %high
    %sum2 = binary add i64 %sum1, %a
    %sum3 = binary add i64 %sum2, %b
    %sum4 = binary add i64 %sum3, %c
    %sum5 = binary add i64 %sum4, %d
    %result = binary add i64 %sum5, %tail
    return i64 %result
}

function @main() -> i64 [role=entry] {
  slot $pair : obj<16x8>

  block ^entry:
    %address = addr $pair
    store i64 100, %address
    %high_address = index i8 %address, 8
    store i64 200, %high_address
    %tail = const i64 50
    %a = const i64 1
    %b = const i64 2
    %c = const i64 3
    %d = const i64 4
    %sum = call i64 @sum_object_and_scalars($pair, %a, %b, %c, %d, %tail)
    %ok = cmp eq i64 %sum, 360
    %failed = cmp eq i64 %ok, 0
    return i64 %failed
}
