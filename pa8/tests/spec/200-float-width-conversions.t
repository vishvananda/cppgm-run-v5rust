function @main() -> i64 {
  block ^entry:
    %0 = convert fpext f64 f32 1.5f
    %1 = cmp eq f64 %0, 1.5
    %2 = convert fpext f80 f64 %0
    %3 = convert fptrunc f32 f80 %2
    %4 = cmp eq f32 %3, 1.5f
    %5 = convert fptrunc f64 f80 %2
    %6 = cmp eq f64 %5, 1.5
    %7 = binary add i64 %1, %4
    %8 = binary add i64 %7, %6
    return i64 %8
}
