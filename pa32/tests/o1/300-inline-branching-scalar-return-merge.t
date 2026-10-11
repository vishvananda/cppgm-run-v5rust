function @pick(%x : i32) -> i32 {
  block ^entry:
    %t2 = cmp eq i32 %x, 0
    branch %t2, ^zero, ^nonzero

  block ^zero:
    return i32 1

  block ^nonzero:
    return i32 0
}
function @caller(%x : i32) -> i32 {
  block ^entry:
    %t2 = call i32 @pick(%x)
    branch %t2, ^then, ^else

  block ^then:
    return i32 11

  block ^else:
    return i32 22
}
