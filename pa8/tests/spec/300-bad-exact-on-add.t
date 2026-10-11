function @f(%a : i64) -> i64 {
  block ^entry:
    %s = binary add exact i64 %a, 1
    return i64 %s
}
