function @positive_infinity() -> f64 [binding=strong] {
  block ^entry:
    return f64 inf
}

function @negative_infinity() -> f32 [binding=strong] {
  block ^entry:
    return f32 -inf
}

function @quiet_nan() -> f64 [binding=strong] {
  block ^entry:
    return f64 nan
}

function @extended_infinity() -> f80 [binding=strong] {
  block ^entry:
    return f80 INFINITY
}

function @suffixed_literal() -> f32 [binding=strong] {
  block ^entry:
    return f32 1.5f
}
