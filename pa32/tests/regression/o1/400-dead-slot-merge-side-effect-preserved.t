declare function @touch() -> void

function @main(%cond : i1) -> void {
  slot $dead : i64

  block ^entry:
    branch %cond, ^left, ^right

  block ^left:
    call void @touch()
    store i64 1, $dead
    jump ^merge

  block ^right:
    store i64 2, $dead
    jump ^merge

  block ^merge:
    %v = load i64 $dead
    return void
}
