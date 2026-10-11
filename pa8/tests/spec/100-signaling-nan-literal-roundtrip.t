# lowir.md: snan is the signalling NaN and keeps that spelling; writing it
# back as nan changes the value
function @f() -> i64 {
  block ^entry:
    %a = const f64 snan
    return i64 0
}
