function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const i64 1 !dbg(test.cpp, 1, 1)
    %b = const i64 2 !dbg(test.cpp, 2, 1)
    %c = binary add i64 %a, %b !dbg(test.cpp, 3, 1)
    return i64 %c !dbg(test.cpp, 4, 1)
}
