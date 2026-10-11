function @one() -> f64 {
  block ^entry:
    return f64 1
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %value = call f64 @one()
    %ok = cmp eq f64 %value, 1.0
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
