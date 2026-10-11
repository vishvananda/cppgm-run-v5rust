function @main() -> i64 [role=entry] {
  slot $x : u16

  block ^entry:
    store u16 65535, $x
    %px = addr $x
    %v = load u16 %px
    %w = copy i64 %v
    %ok = cmp eq i64 %w, 65535
    return i64 %ok
}
