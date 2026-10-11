function @main() -> i64 [role=entry] {
  block ^entry:
    %a = cmp ult u16 7, 8
    branch %a, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
