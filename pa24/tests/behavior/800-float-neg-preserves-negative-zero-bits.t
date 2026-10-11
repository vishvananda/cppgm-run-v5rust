function @main() -> i32 [role=entry, binding=strong] {
  slot $box : f64

  block ^entry:
    %zero = const f64 0.0
    %neg = unary neg f64 %zero
    %p = addr $box
    store f64 %neg, %p
    %raw = load i64 %p
    %ok = cmp eq i64 %raw, -9223372036854775808
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
