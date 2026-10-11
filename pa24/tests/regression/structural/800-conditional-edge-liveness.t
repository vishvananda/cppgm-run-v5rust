function @f(%cond : i64, %x : i64) -> i64 {
  block ^entry:
    %t0 = copy i64 %x
    %cmp = cmp ne i64 %cond, 0
    branch %cmp, ^then, ^else

  block ^then:
    return i64 %t0

  block ^else:
    return i64 0
}

function @__cppgm_406d61696e() -> i32 [role=entry, binding=strong] {
  block ^entry:
    %cond = const i64 1
    %x = const i64 41
    %y = call i64 @f(%cond, %x)
    %ok = cmp eq i64 %y, 41
    branch %ok, ^good, ^bad

  block ^good:
    return i32 0

  block ^bad:
    return i32 1
}
