function @main() -> i64 [role=entry] {
  block ^entry:
    %a = convert uitofp f64 u32 7
    %b = convert fptoui u32 f64 %a
    %c = cmp eq u32 %b, 7
    branch %c, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
