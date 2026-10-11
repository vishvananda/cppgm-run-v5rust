function @eh_loop(%n : i64, %cleanups : ptr) -> i64 [binding=strong, object=_Z7eh_looplPl] {
  block ^entry:
    %__o1inl1__t3 = copy ptr %cleanups
    jump ^for_cond_1

  block ^for_cond_1:
    %t0 = phi i64 [^entry: 0, ^for_body_2: %t16]
    %t22 = phi i64 [^entry: 0, ^for_body_2: %t20]
    %t3 = cmp lt i64 %t22, %n
    branch %t3, ^for_body_2, ^for_end_4

  block ^for_body_2:
    %t10 = call i64 @work(%t22)
    %t13 = binary shl nowrap i64 %t10, 1
    %t15 = binary add nowrap i64 %t13, %t22
    %t16 = binary add nowrap i64 %t0, %t15
    %__o1inl1__t4 = load typed i64 %__o1inl1__t3
    %__o1inl1__t6 = binary add nowrap i64 %__o1inl1__t4, 1
    store typed i64 %__o1inl1__t6, %__o1inl1__t3
    %t20 = binary add nowrap i64 %t22, 1
    jump ^for_cond_1

  block ^for_end_4:
    return i64 %t0
}
function @work(%value : i64) -> i64 [binding=strong, object=_Z4workl, no_inline=yes] {
  block ^entry:
    %t2 = binary add nowrap i64 %value, 3
    return i64 %t2
}
function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  slot $cleanups : i64

  block ^entry:
    store typed i64 0, $cleanups
    %t1 = addr $cleanups
    %t2 = call i64 @eh_loop(4, %t1)
    %t3 = cmp eq i64 %t2, 42
    branch %t3, ^land_rhs_4, ^land_short_5

  block ^land_rhs_4:
    %t4 = load typed i64 $cleanups
    %t5 = cmp eq i64 %t4, 4
    jump ^land_end_6

  block ^land_short_5:
    jump ^land_end_6

  block ^land_end_6:
    %t9 = phi i64 [^land_rhs_4: %t5, ^land_short_5: 0]
    branch %t9, ^cond_then_1, ^cond_else_2

  block ^cond_then_1:
    jump ^cond_end_3

  block ^cond_else_2:
    jump ^cond_end_3

  block ^cond_end_3:
    %t0 = phi i32 [^cond_then_1: 0, ^cond_else_2: 1]
    return i32 %t0
}
