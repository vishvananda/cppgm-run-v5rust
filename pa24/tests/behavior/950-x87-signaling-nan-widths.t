function @bits32(%value : f32, %expected : i32) -> i64 {
  slot $cell : i32
  block ^entry:
    %address = addr $cell
    store f32 %value, %address
    %bits = load i32 %address
    %same = cmp eq i32 %bits, %expected
    return i64 %same
}
function @bits64(%value : f64, %expected : i64) -> i64 {
  slot $cell : i64
  block ^entry:
    %address = addr $cell
    store f64 %value, %address
    %bits = load i64 %address
    %same = cmp eq i64 %bits, %expected
    return i64 %same
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %a = call i64 @bits32(snanL, 2141192192)
    %b = call i64 @bits32(-snanL, -6291456)
    %c = call i64 @bits64(snanL, 9219994337134247936)
    %d = call i64 @bits64(-snanL, -3377699720527872)
    %ab = binary add i64 %a, %b
    %cd = binary add i64 %c, %d
    %total = binary add i64 %ab, %cd
    %bad = cmp ne i64 %total, 4
    return i64 %bad
}
