function @main() -> i64 [role=entry] {
  slot $op : i32

  block ^entry:
    store i32 6, $op
    %v = load i32 $op
    %ok = cmp eq i32 %v, 6
    branch %ok, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
