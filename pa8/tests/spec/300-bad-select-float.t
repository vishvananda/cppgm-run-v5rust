function @f(%c : i64, %a : f64, %b : f64) -> f64 {
  block ^entry:
    %v = select f64 %c, %a, %b
    return f64 %v
}
