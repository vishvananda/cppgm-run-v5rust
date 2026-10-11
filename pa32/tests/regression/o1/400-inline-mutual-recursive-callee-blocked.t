function @ping(%n : i64) -> i64 {
  block ^entry:
    %t1 = cmp eq i64 %n, 0
    branch %t1, ^base, ^step

  block ^base:
    return i64 %n

  block ^step:
    %t2 = binary sub i64 %n, 1
    %t3 = call i64 @pong(%t2)
    return i64 %t3
}

function @pong(%n : i64) -> i64 {
  block ^entry:
    %t1 = cmp eq i64 %n, 0
    branch %t1, ^base, ^step

  block ^base:
    return i64 %n

  block ^step:
    %t2 = binary sub i64 %n, 1
    %t3 = call i64 @ping(%t2)
    return i64 %t3
}

function @caller(%x : i64) -> i64 {
  block ^entry:
    %t1 = call i64 @ping(%x)
    %t2 = binary add i64 %t1, 1
    return i64 %t2
}
