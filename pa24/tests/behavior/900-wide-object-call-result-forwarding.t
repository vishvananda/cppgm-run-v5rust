function @make_pair() -> obj<16x8> {
  slot $value : obj<16x8>

  block ^entry:
    %address = addr $value
    store i64 11, %address
    %high_address = index i8 %address, 8
    store i64 22, %high_address
    return obj<16x8> $value
}

function @forward_pair() -> obj<16x8> {
  block ^entry:
    %value = call obj<16x8> @make_pair()
    return obj<16x8> %value
}

function @main() -> i64 [role=entry] {
  slot $result : obj<16x8>

  block ^entry:
    %value = call obj<16x8> @forward_pair()
    %destination = addr $result
    copyobj 16x8 %value, %destination
    %low = load i64 %destination
    %high_address = index i8 %destination, 8
    %high = load i64 %high_address
    %low_ok = cmp eq i64 %low, 11
    %high_ok = cmp eq i64 %high, 22
    %ok = binary and i64 %low_ok, %high_ok
    %failed = cmp eq i64 %ok, 0
    return i64 %failed
}
