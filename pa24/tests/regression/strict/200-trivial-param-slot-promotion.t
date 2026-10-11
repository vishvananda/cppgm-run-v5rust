global @cell = {
  i64 7
}

function @read(%p : ptr) -> i64 {
  slot $p : ptr

  block ^entry:
    store ptr %p, $p
    %t1 = load ptr $p
    %t2 = load i64 %t1
    return i64 %t2
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %t1 = addr @cell
    %t2 = call i64 @read(%t1)
    return i64 %t2
}
