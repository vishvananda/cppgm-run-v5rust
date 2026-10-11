function @id(%x : i64) -> i64 {
  block ^entry:
    return i64 %x !dbg(test.cpp, 1, 1)
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %x = const i64 20 !dbg(test.cpp, 5, 1)
    %y = call i64 @id(%x) !dbg(test.cpp, 6, 1)
    return i64 %y !dbg(test.cpp, 7, 1)
}
