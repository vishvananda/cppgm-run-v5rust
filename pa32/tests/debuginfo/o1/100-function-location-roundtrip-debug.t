function @probe(%arg : i64) -> i64 !dbg(calc.cpp, 1, 1) {
  block ^entry:
    %sum = binary add i64 %arg, 0 !dbg(calc.cpp, 10, 6)
    return i64 %sum !dbg(calc.cpp, 11, 2)
}
