function @add(%a : f64, %b : f64) -> f64 {
  block ^entry:
    %sum = binary add f64 %a, %b
    return f64 %sum
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %a = const f64 1.5
    %b = const f64 2.25
    %r = call f64 @add(%a, %b)
    %e = const f64 3.75
    %ok = cmp eq f64 %r, %e
    return i64 %ok
}
