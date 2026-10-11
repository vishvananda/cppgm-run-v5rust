function @f() -> obj<3x1> [binding=strong] {
  slot $x : obj<3x1>

  block ^entry:
    %t1 = addr $x
    zeroinit 3x1 %t1
    %t2 = index i8 [projection=field] %t1, 1
    store u8 1, %t2
    return obj<3x1> $x
}
function @__cppgm_406d61696e() -> i32 [role=entry, binding=strong] {
  slot $out : obj<3x1>

  block ^entry:
    %t1 = addr $out
    %t2 = call obj<3x1> @f()
    copyobj 3x1 %t2, %t1
    return i32 0
}
