function @main() -> i64 {
  slot $tmp : i64
  block ^entry:
    %base = addr $tmp
    %field = index i8 [projection=diagonal] %base, 4
    return i64 0
}
