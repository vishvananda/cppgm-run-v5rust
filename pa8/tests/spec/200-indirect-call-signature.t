function @helper(%x : i64) -> i64 {
  block ^entry:
    return i64 %x
}

function @main() -> i64 {
  block ^entry:
    %fp = addr @helper
    %0 = call i64 %fp(7) as (%arg0 : i64) -> i64
    return i64 %0
}
