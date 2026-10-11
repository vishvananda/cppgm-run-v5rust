function @id(%p : ptr) -> i64 {
  block ^entry:
    %v = load i32 %p !dbg(test.cpp, 1, 1)
    return i64 %v !dbg(test.cpp, 2, 1)
}
function @main() -> i64 [role=entry] {
  slot $x : i64

  block ^entry:
    %p1 = addr $x !dbg(test.cpp, 6, 1)
    store i32 7, %p1 !dbg(test.cpp, 7, 1)
    %p2 = addr $x !dbg(test.cpp, 8, 1)
    %r = call i64 @id(%p2) !dbg(test.cpp, 9, 1)
    return i64 %r !dbg(test.cpp, 10, 1)
}
