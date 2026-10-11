function @alpha(%n : i64) -> i64 {
  block ^entry:
    %t1 = cmp eq i64 %n, 0
    branch %t1, ^base, ^step

  block ^base:
    return i64 %n

  block ^step:
    %t2 = binary sub i64 %n, 1
    %t3 = call i64 @beta(%t2)
    return i64 %t3
}
function @beta(%n : i64) -> i64 {
  block ^entry:
    %t1 = cmp eq i64 %n, 0
    branch %t1, ^base, ^step

  block ^base:
    return i64 %n

  block ^step:
    %t2 = binary sub i64 %n, 1
    %t3 = call i64 @gamma(%t2)
    return i64 %t3
}
function @gamma(%n : i64) -> i64 {
  block ^entry:
    %t1 = cmp eq i64 %n, 0
    branch %t1, ^base, ^step

  block ^base:
    return i64 %n

  block ^step:
    %t2 = binary sub i64 %n, 1
    %t3 = call i64 @alpha(%t2)
    return i64 %t3
}
function @caller(%x : i64) -> i64 {
  block ^entry:
    %t1 = call i64 @alpha(%x)
    %t2 = binary add i64 %t1, 1
    return i64 %t2
}
