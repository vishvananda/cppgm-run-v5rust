function @main() -> i32 [role=entry, binding=strong] {
  block ^entry:
    %value = convert fptrunc f64 f80 1.0L
    %ok = cmp eq f64 %value, 1.0
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
