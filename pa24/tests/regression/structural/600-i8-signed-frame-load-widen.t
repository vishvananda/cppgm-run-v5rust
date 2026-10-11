function @main() -> i64 [role=entry] {
  slot $x : i8

  block ^entry:
    store i8 -1, $x
    %px = addr $x
    %v = load i8 %px
    %w = copy i64 %v
    %ok = cmp eq i64 %w, -1
    return i64 %ok
}
