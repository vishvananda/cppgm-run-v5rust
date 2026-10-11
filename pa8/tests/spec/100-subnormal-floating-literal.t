# N3485 2.14.4p1 (as lowir.md reads floating literals): an in-range decimal
# literal rounds to the nearest representable value; the subnormal 4.9e-324 is
# valid
function @f() -> i64 {
  block ^entry:
    %a = const f64 4.9e-324
    return i64 0
}
