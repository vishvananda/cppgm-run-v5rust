function @main() -> i64 {
  block ^entry:
    %0 = const i64 9
    %1 = copy i64 %0
    return i64 %1
}
