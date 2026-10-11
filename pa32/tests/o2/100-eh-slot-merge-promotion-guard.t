declare function @may_throw() -> void [binding=strong]

function @f() -> i64 [binding=strong] {
  slot $x : i64

  block ^entry:
    store i64 0, $x
    eh_try ^catch_1
    call void @may_throw()
    store i64 1, $x
    eh_end
    jump ^after

  block ^catch_1:
    store i64 2, $x
    jump ^after

  block ^after:
    %t1 = load i64 $x
    return i64 %t1
}
