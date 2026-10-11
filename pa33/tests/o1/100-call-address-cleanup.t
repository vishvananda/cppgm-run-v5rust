function @id(%p : ptr) -> i64 {
  block ^entry:
    %v = load i32 %p
    return i64 %v
}
function @main() -> i64 [role=entry] {
  slot $x : i64

  block ^entry:
    %p1 = addr $x
    store i32 7, %p1
    %p2 = addr $x
    %r = call i64 @id(%p2)
    return i64 %r
}
