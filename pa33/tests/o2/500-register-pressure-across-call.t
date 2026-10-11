function @pressure(%n : i64) -> i64 [binding=strong, object=_Z8pressurel] {
  block ^entry:
    jump ^for_cond_1

  block ^for_cond_1:
    %t0 = phi i64 [^entry: 1, ^for_body_2: %t8]
    %t47 = phi i64 [^entry: 2, ^for_body_2: %t11]
    %t48 = phi i64 [^entry: 3, ^for_body_2: %t14]
    %t49 = phi i64 [^entry: 4, ^for_body_2: %t17]
    %t50 = phi i64 [^entry: 5, ^for_body_2: %t20]
    %t51 = phi i64 [^entry: 6, ^for_body_2: %t23]
    %t52 = phi i64 [^entry: 7, ^for_body_2: %t26]
    %t53 = phi i64 [^entry: 8, ^for_body_2: %t29]
    %t54 = phi i64 [^entry: 0, ^for_body_2: %t31]
    %t3 = cmp lt i64 %t54, %n
    branch %t3, ^for_body_2, ^for_end_4

  block ^for_body_2:
    %t5 = call i64 @step(%t54)
    %t8 = binary add nowrap i64 %t0, %t5
    %t11 = binary add nowrap i64 %t47, %t8
    %t14 = binary add nowrap i64 %t48, %t11
    %t17 = binary add nowrap i64 %t49, %t14
    %t20 = binary add nowrap i64 %t50, %t17
    %t23 = binary add nowrap i64 %t51, %t20
    %t26 = binary add nowrap i64 %t52, %t23
    %t29 = binary add nowrap i64 %t53, %t26
    %t31 = binary add nowrap i64 %t54, 1
    jump ^for_cond_1

  block ^for_end_4:
    %t34 = binary add nowrap i64 %t0, %t47
    %t36 = binary add nowrap i64 %t34, %t48
    %t38 = binary add nowrap i64 %t36, %t49
    %t40 = binary add nowrap i64 %t38, %t50
    %t42 = binary add nowrap i64 %t40, %t51
    %t44 = binary add nowrap i64 %t42, %t52
    %t46 = binary add nowrap i64 %t44, %t53
    return i64 %t46
}
function @step(%value : i64) -> i64 [binding=strong, object=_Z4stepl, no_inline=yes] {
  block ^entry:
    %t2 = binary add nowrap i64 %value, 1
    return i64 %t2
}
function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %t1 = call i64 @pressure(3)
    %t2 = cmp eq i64 %t1, 1008
    branch %t2, ^cond_then_1, ^cond_else_2

  block ^cond_then_1:
    jump ^cond_end_3

  block ^cond_else_2:
    jump ^cond_end_3

  block ^cond_end_3:
    %t0 = phi i32 [^cond_then_1: 0, ^cond_else_2: 1]
    return i32 %t0
}
