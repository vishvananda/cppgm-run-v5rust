declare function @observe(%arg0 : i32) -> i32 [binding=strong, object=_Z7observei]

function @with_cleanup(%values : ptr, %count : i32, %slot : ptr) -> i32 [binding=strong, object=_Z12with_cleanupPKiiPi] {
  slot $values : ptr
  slot $count : i32
  slot $slot : ptr
  slot $guard : obj<8x8>
  slot $total : i32
  slot $i : i32
  slot $call__1 : i32

  block ^entry:
    store typed ptr %values, $values
    store typed i32 %count, $count
    store typed ptr %slot, $slot
    %t1 = addr $guard
    %t2 = load typed ptr $slot
    %t3 = addr $guard
    %t4 = index i8 [projection=field] %t3, 0
    store typed ptr %t2, %t4
    store typed i32 0, $total
    store typed i32 0, $i
    jump ^for_cond_1

  block ^for_cond_1:
    %t5 = load typed i32 $i
    %t6 = load typed i32 $count
    %t7 = cmp lt i32 %t5, %t6
    branch %t7, ^for_body_2, ^for_end_4

  block ^for_body_2:
    %t8 = load typed i32 $total
    eh_try ^cleanup_action_6
    %t9 = load typed ptr $values
    %t10 = load typed i32 $i
    %t11 = index i32 [projection=array_element] %t9, %t10
    %t12 = load typed i32 %t11
    %t13 = call i32 @observe(%t12)
    store typed i32 %t13, $call__1
    %t14 = load typed i32 $call__1
    %t15 = load typed i32 $i
    %t16 = binary add nowrap i32 %t14, %t15
    %t17 = binary add nowrap i32 %t8, %t16
    store typed i32 %t17, $total
    eh_end
    jump ^call_unwind_end_7

  block ^cleanup_resume_5:
    resume

  block ^cleanup_action_6:
    %t18 = addr $guard
    call void @Guard___Guard(%t18)
    jump ^cleanup_resume_5

  block ^call_unwind_end_7:
    jump ^for_iter_3

  block ^for_iter_3:
    %t19 = load typed i32 $i
    %t20 = binary add nowrap i32 %t19, 1
    store typed i32 %t20, $i
    jump ^for_cond_1

  block ^for_end_4:
    %t21 = load typed i32 $total
    %t22 = addr $guard
    call void @Guard___Guard(%t22)
    return i32 %t21
}
function @Guard___Guard(%this : ptr [object_bytes=8]) -> void [unwind=no, binding=weak, object=_ZN5GuardD1Ev, inline_hint=yes] {
  slot $this : ptr

  block ^entry:
    store typed ptr %this, $this
    %t1 = load typed ptr $this
    %t2 = index i8 [projection=field] %t1, 0
    %t3 = load typed ptr %t2
    %t4 = load typed i32 %t3
    %t5 = binary add nowrap i32 %t4, 1
    store typed i32 %t5, %t3
    return void
}
alias object _ZN5GuardD2Ev = @Guard___Guard
