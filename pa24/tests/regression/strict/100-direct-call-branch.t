function @helper(%x : i64) -> i64 {
  block ^entry:
    return i64 %x
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %c = const i64 1
    branch %c, ^then, ^else

  block ^then:
    %x = const i64 7
    %r = call i64 @helper(%x)
    return i64 %r

  block ^else:
    return i64 0
}
