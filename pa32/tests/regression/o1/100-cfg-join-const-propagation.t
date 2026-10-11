function @ping() -> i64 {
  block ^entry:
    return i64 9
}
function @main(%cond : i64) -> i64 {
  block ^entry:
    %x = copy i64 4
    branch %cond, ^left, ^right

  block ^left:
    %l = call i64 @ping()
    jump ^merge

  block ^right:
    %r = call i64 @ping()
    jump ^merge

  block ^merge:
    %y = binary add i64 %x, 3
    return i64 %y
}
