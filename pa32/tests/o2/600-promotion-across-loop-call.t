declare function @observe(%arg0 : i32) -> i32 [binding=strong, object=_Z7observei]

function @call_in_loop(%values : ptr, %count : i32, %scale : i32) -> i32 [binding=strong, object=_Z12call_in_loopPKiii] {
  slot $values : ptr
  slot $count : i32
  slot $scale : i32
  slot $total : i32
  slot $i : i32
  slot $scaled : i32

  block ^entry:
    store typed ptr %values, $values
    store typed i32 %count, $count
    store typed i32 %scale, $scale
    store typed i32 0, $total
    store typed i32 0, $i
    jump ^for_cond_1

  block ^for_cond_1:
    %t1 = load typed i32 $i
    %t2 = load typed i32 $count
    %t3 = cmp lt i32 %t1, %t2
    branch %t3, ^for_body_2, ^for_end_4

  block ^for_body_2:
    %t4 = load typed ptr $values
    %t5 = load typed i32 $i
    %t6 = index i32 [projection=array_element] %t4, %t5
    %t7 = load typed i32 %t6
    %t8 = load typed i32 $scale
    %t9 = binary mul nowrap i32 %t7, %t8
    store typed i32 %t9, $scaled
    %t10 = load typed i32 $total
    %t11 = load typed i32 $scaled
    %t12 = call i32 @observe(%t11)
    %t13 = load typed i32 $scaled
    %t14 = binary add nowrap i32 %t12, %t13
    %t15 = binary add nowrap i32 %t10, %t14
    store typed i32 %t15, $total
    jump ^for_iter_3

  block ^for_iter_3:
    %t16 = load typed i32 $i
    %t17 = binary add nowrap i32 %t16, 1
    store typed i32 %t17, $i
    jump ^for_cond_1

  block ^for_end_4:
    %t18 = load typed i32 $total
    return i32 %t18
}
