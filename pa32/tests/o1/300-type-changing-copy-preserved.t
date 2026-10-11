function @main(%bits : i64) -> i64 {
  block ^entry:
    %address = copy ptr %bits
    store i32 0, %address
    return i64 0
}
