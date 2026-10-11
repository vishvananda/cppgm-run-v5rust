function @scale(%x : f64) -> f64 {
  block ^entry:
    %0 = const f64 2.0
    %1 = binary mul f64 %x, %0
    return f64 %1
}

function @main() -> i64 {
  block ^entry:
    %0 = const f64 3.0
    %1 = call f64 @scale(%0)
    %2 = const f64 6.0
    %3 = cmp eq f64 %1, %2
    return i64 %3
}
