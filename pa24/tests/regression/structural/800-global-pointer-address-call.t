global @fp : ptr = addr @ret3

function @ret3() -> i64 {
  block ^entry:
    return i64 3
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %p = addr @fp
    %r = call i64 %p() as () -> i64
    return i64 %r
}
