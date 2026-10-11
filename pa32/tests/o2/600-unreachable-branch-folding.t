declare function @observe(%arg0 : i32) -> i32 [binding=strong, object=_Z7observei]

function @after_unreachable(%x : i32) -> i32 [binding=strong, object=_Z17after_unreachablei] {
  slot $x : i32
  slot $doubled : i32

  block ^entry:
    store typed i32 %x, $x
    %t1 = load typed i32 $x
    %t2 = cmp gt i32 %t1, 10
    branch %t2, ^if_then_1, ^if_else_2

  block ^if_then_1:
    unreachable

  block ^if_else_2:
    jump ^if_end_3

  block ^if_end_3:
    %t3 = load typed i32 $x
    %t4 = binary mul nowrap i32 %t3, 2
    store typed i32 %t4, $doubled
    %t5 = load typed i32 $doubled
    %t6 = cmp gt i32 %t5, 100
    branch %t6, ^if_then_4, ^if_else_5

  block ^if_then_4:
    %t7 = load typed i32 $doubled
    %t8 = call i32 @observe(%t7)
    return i32 %t8

  block ^if_else_5:
    jump ^if_end_6

  block ^if_end_6:
    %t9 = load typed i32 $doubled
    %t10 = binary add nowrap i32 %t9, 1
    return i32 %t10
}
