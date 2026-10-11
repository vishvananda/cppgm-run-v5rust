global @table = {
  f80 4.5L
  zero 16
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %p = addr @table
    %x = load f80 %p
    %y = const f80 4.5L
    %ok = cmp eq f80 %x, %y
    return i64 %ok
}
