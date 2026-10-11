function @id(%x : i64) -> i64 {
  block ^entry:
    return i64 %x
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %x = const i64 20
    %y = call i64 @id(%x)
    return i64 %y
}
