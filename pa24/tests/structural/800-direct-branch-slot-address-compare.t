function @main() -> i64 [role=entry] {
  slot $x : i64

  block ^entry:
    store i64 0, $x
    %p = addr $x
    %q = addr $x
    %ok = cmp eq ptr %p, %q
    branch %ok, ^same, ^different

  block ^same:
    return i64 0

  block ^different:
    return i64 1
}
