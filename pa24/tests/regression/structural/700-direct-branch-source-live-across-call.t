function @one() -> i64 {
  block ^entry:
    return i64 1
}

function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64) -> i64 {
  block ^entry:
    return i64 0
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %x = call i64 @one()
    %ok = cmp eq i64 %x, 1
    %ignored = call i64 @clobber(10, 20, 30, 40, 2, 2)
    branch %ok, ^true, ^false

  block ^true:
    return i64 0

  block ^false:
    return i64 1
}
