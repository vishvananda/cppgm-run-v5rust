function @main() -> i64 {
  block ^entry:
    %0 = const i64 1
    %1 = const i64 2
    %2 = cmp lt i64 %0, %1
    branch %2, ^then, ^else

  block ^then:
    %3 = const i64 1
    return i64 %3

  block ^else:
    %4 = const i64 2
    return i64 %4
}
