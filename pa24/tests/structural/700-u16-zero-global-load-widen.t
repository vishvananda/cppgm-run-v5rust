global @g : u16 = 65535

function @main() -> i64 [role=entry] {
  block ^entry:
    %v = load u16 @g
    %w = copy i64 %v
    %ok = cmp eq i64 %w, 65535
    return i64 %ok
}
