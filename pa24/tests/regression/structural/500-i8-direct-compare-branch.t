function @main() -> i64 [role=entry] {
  block ^entry:
    %a = cmp eq i8 7, 7
    branch %a, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
