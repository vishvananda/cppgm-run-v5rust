function @pick(%ret : ptr [pass=indirect_result], %cond : i64, %lhs : ptr, %rhs : ptr) -> void {
  block ^entry:
    branch %cond, ^left, ^right

  block ^left:
    %a = load i64 %lhs
    store i64 %a, %ret
    return void

  block ^right:
    %b = load i64 %rhs
    store i64 %b, %ret
    return void
}
function @main(%cond : i64, %lhs : ptr, %rhs : ptr) -> i64 {
  slot $value : i64

  block ^entry:
    %p = addr $value
    call void @pick(%p, %cond, %lhs, %rhs)
    %r = load i64 $value
    return i64 %r
}
