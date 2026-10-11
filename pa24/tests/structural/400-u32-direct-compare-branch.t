function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = cmp ult u32 1, 2
    branch %0, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
