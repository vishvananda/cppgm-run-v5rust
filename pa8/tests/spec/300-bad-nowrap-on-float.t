function @f(%a : f64) -> f64 {
  block ^entry:
    %s = binary add nowrap f64 %a, 1.0
    return f64 %s
}
