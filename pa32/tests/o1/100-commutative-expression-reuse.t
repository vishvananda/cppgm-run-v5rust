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
    %a = binary add i64 %x, %y
    %b = binary add i64 %y, %x
    %c = binary mul i64 %a, %b
    return i64 %c
}
function @cfg(%x : i64, %y : i64, %cond : i64) -> i64 {
  block ^entry:
    %a = binary add i64 %x, %y
    branch %cond, ^left, ^right

  block ^left:
    %l = call i64 @ping()
    jump ^merge

  block ^right:
    %r = call i64 @pong()
    jump ^merge

  block ^merge:
    %b = binary add i64 %y, %x
    return i64 %b
}
