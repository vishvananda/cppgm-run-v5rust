function @main() -> i64 !dbg(main.cpp, 1, 1) {
  block ^entry:
    %zero = const i64 0 !dbg(main.cpp, 2, 3)
    return i64 %zero !dbg(main.cpp, 3, 3)
}
