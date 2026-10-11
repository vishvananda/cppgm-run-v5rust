function @choose(%condition : i64) -> i64 {
  slot $value : i64

  block ^entry:
    store i64 4294967295, $value
    %loaded = load i64 $value
    %take_inverted = cmp ne i64 %condition, 0
    branch %take_inverted, ^inverted, ^plain

  block ^inverted:
    %result = unary bitnot i64 %loaded
    return i64 %result

  block ^plain:
    return i64 %loaded
}
