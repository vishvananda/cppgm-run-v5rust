function @f(%x : i32) -> i32 {
  slot $y : i32

  block ^entry:
    store i32 %x, $y !dbg(calc.cpp, 2, 3)
    jump ^use !dbg(calc.cpp, 2, 9)

  block ^use:
    %1 = load i32 $y !dbg(calc.cpp, 3, 3)
    %2 = convert sext i64 i32 %1 !dbg(calc.cpp, 4, 3)
    %3 = binary add i64 %2, 1 !dbg(calc.cpp, 5, 3)
    %4 = convert trunc i32 i64 %3 !dbg(calc.cpp, 6, 3)
    return i32 %4 !dbg(calc.cpp, 7, 3)
}
