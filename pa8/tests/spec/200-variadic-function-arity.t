function @sum(%count : i64) -> i64 [arity=variadic] {
  block ^entry:
    return i64 %count
}

function @main() -> i64 {
  block ^entry:
    %0 = call i64 @sum(7)
    return i64 %0
}
