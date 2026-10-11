function @main() -> i32 [role=entry, binding=strong] {
  block ^entry:
    %t1 = binary add i64 0, 3472328296227680304
    %t2 = cmp ne i64 %t1, 3472328296227680304
    branch %t2, ^bad_add, ^check_sub

  block ^bad_add:
    return i32 1

  block ^check_sub:
    %t3 = binary sub i64 3472328296227680304, 3472328296227680304
    %t4 = cmp ne i64 %t3, 0
    branch %t4, ^bad_sub, ^ok

  block ^bad_sub:
    return i32 2

  block ^ok:
    return i32 0
}
