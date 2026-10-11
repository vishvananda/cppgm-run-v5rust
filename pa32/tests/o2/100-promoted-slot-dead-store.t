function @main(%x : i64, %y : i64) -> i64 {
  slot $s : i64

  block ^entry:
    store i64 %x, $s
    store i64 %y, $s
    %z = load i64 $s
    return i64 %z
}
