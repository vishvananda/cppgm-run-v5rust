function @main(%condition : i128) -> i64 [role=entry] {
  block ^entry:
    %result = select i64 %condition, 1, 0
    return i64 %result
}
