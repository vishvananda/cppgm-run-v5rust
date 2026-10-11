function @accept(%value : i128) -> i64 {
  block ^entry:
    %is_zero = cmp eq i128 %value, 0
    %bad = cmp eq i64 %is_zero, 0
    return i64 %bad
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %result = call i64 @accept(0)
    return i64 %result
}
