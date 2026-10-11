function @helper() -> i64 [binding=strong] {
  block ^entry:
    %0 = const i64 7
    return i64 %0
}
