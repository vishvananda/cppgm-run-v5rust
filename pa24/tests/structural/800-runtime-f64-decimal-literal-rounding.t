function @main() -> i32 [role=entry, binding=strong] {
  block ^entry:
    %decimal = const f64 623e+100
    %rounded = const f64 0x1.640a62f3a83dfp+341
    %same = cmp eq f64 %decimal, %rounded
    branch %same, ^ok, ^bad

  block ^bad:
    return i32 1

  block ^ok:
    return i32 0
}
