function @main() -> i64 [role=entry] {
  block ^entry:
    %p = const ptr 0
    %ok = cmp eq ptr %p, 0
    branch %ok, ^good, ^bad

  block ^bad:
    return i64 0

  block ^good:
    return i64 1
}
