function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = cmp eq f64 3.0, 3.0
    branch %0, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
