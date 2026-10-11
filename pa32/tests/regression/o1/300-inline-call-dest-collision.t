function @callee(%x : i32) -> i32 {
  block ^entry:
    %t2 = cmp eq i32 %x, 0
    %t3 = cmp ne i64 %t2, 0
    return i32 %t3
}
function @caller(%x : i32) -> i32 {
  block ^entry:
    %t2 = call i32 @callee(%x)
    branch %t2, ^then, ^else

  block ^then:
    return i32 1

  block ^else:
    return i32 0
}
