function @probe(%arg : i64) -> i64 {
  block ^entry:
    %sum = binary add i64 %arg, 0 !dbg(calc.cpp, 10, 6)
    %cmp = cmp eq i64 %sum, %sum !dbg(calc.cpp, 11, 4)
    branch %cmp, ^ok, ^ok !dbg(calc.cpp, 12, 2)

  block ^ok:
    return i64 %sum !dbg(calc.cpp, 13, 2)
}
