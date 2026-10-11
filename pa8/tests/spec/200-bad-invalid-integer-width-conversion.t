function @main() -> i64 {
  block ^entry:
    %0 = convert zext i32 i64 1
    return i64 %0
}
