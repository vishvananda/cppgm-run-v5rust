global @g : i8 = -1

function @main() -> i64 [role=entry] {
  block ^entry:
    %v = load i8 @g
    %w = copy i64 %v
    %ok = cmp eq i64 %w, -1
    return i64 %ok
}
