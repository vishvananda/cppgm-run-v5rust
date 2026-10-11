function @zero() -> i64 {
  block ^entry:
    return i64 0 !dbg(test.cpp, 1, 1)
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %x = call i64 @zero() !dbg(test.cpp, 5, 1)
    %ok = unary not i64 %x !dbg(test.cpp, 6, 1)
    branch %ok, ^true, ^false !dbg(test.cpp, 7, 1)

  block ^true:
    return i64 0 !dbg(test.cpp, 10, 1)

  block ^false:
    return i64 1 !dbg(test.cpp, 13, 1)
}
