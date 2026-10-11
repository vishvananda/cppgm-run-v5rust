function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const f64 1.5
    %b = copy f64 %a
    %c = binary add f64 %b, 2.25
    %ok = cmp eq f64 %c, 3.75
    branch %ok, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
