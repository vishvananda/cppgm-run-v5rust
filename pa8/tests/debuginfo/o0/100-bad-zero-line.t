function @probe() -> i64 {
  block ^entry:
    %seed = const i64 42 !dbg(main.cpp, 0, 7)
    return i64 %seed
}
