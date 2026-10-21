global @value : i64 = 7

function @address_f32() -> f32 {
  block ^entry:
    return f32 @value
}

function @address_f64() -> f64 {
  block ^entry:
    %address = copy f64 @value
    return f64 %address
}

function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
