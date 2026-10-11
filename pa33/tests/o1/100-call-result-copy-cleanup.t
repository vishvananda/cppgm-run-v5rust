function @zero() -> i64 {
  block ^entry:
    return i64 0
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %x = call i64 @zero()
    %ok = unary not i64 %x
    branch %ok, ^true, ^false

  block ^true:
    return i64 0

  block ^false:
    return i64 1
}
