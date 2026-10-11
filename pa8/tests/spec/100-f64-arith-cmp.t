function @main() -> i64 {
  block ^entry:
    %a = const f64 8.0
    %b = const f64 2.0
    %c = binary div f64 %a, %b
    %d = const f64 4.0
    %ok = cmp eq f64 %c, %d
    return i64 %ok
}
