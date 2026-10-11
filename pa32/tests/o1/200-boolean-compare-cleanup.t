function @ping() -> i64 {
  block ^entry:
    return i64 1
}
function @pong() -> i64 {
  block ^entry:
    return i64 2
}
function @local(%x : i64, %y : i64) -> i64 {
  block ^entry:
    %a = cmp lt i64 %x, %y
    %b = cmp ne i64 %a, 0
    %c = cmp eq i64 %a, 1
    %d = binary add i64 %b, %c
    return i64 %d
}
function @cfg(%x : i64, %y : i64, %cond : i64) -> i64 {
  block ^entry:
    %a = cmp lt i64 %x, %y
    branch %cond, ^left, ^right

  block ^left:
    %l = call i64 @ping()
    jump ^merge

  block ^right:
    %r = call i64 @pong()
    jump ^merge

  block ^merge:
    %b = cmp eq i64 %a, 1
    return i64 %b
}
