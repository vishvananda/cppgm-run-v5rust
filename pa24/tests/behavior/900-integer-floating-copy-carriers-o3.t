function @raw32(%bits : i32) -> f32 {
  block ^entry:
    %value = copy f32 %bits
    return f32 %value
}
function @raw64(%bits : i64) -> f64 {
  block ^entry:
    %value = copy f64 %bits
    return f64 %value
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %a = call f32 @raw32(1065353216)
    %b = call f64 @raw64(4607182418800017408)
    %bits = const i64 4611686018427387904
    %c = copy f64 %bits
    %bad_a = cmp ne f32 %a, 1.0
    %bad_b = cmp ne f64 %b, 1.0
    %bad_c = cmp ne f64 %c, 2.0
    %bad_ab = binary or i64 %bad_a, %bad_b
    %bad = binary or i64 %bad_ab, %bad_c
    return i64 %bad
}
