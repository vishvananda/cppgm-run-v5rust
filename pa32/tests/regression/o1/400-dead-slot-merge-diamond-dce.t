function @main(%cond : i1) -> void {
  slot $dead : i64

  block ^entry:
    branch %cond, ^left, ^right

  block ^left:
    %l = binary add i64 40, 2
    store i64 %l, $dead
    jump ^merge

  block ^right:
    %r = binary mul i64 6, 7
    store i64 %r, $dead
    jump ^merge

  block ^merge:
    %v = load i64 $dead
    return void
}
