function @main(%outer : i64, %inner : i64) -> i64 {
  block ^entry:
    %result = call i64 @wrapper(%outer, %inner)
    %adjusted = binary add i64 %result, 1
    return i64 %adjusted
}

function @wrapper(%outer : i64, %inner : i64) -> i64 {
  block ^entry:
    %is_zero = cmp eq i64 %outer, 0
    branch %is_zero, ^zero, ^nonzero

  block ^zero:
    %from_leaf = call i64 @leaf(%inner)
    return i64 %from_leaf

  block ^nonzero:
    return i64 %outer
}

function @leaf(%inner : i64) -> i64 {
  block ^entry:
    %is_zero = cmp eq i64 %inner, 0
    branch %is_zero, ^zero, ^nonzero

  block ^zero:
    return i64 10

  block ^nonzero:
    return i64 20
}
