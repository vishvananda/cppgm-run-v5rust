global @fp : ptr = addr @ret3

function @ret3() -> i64 {
  block ^entry:
    return i64 3
}

function @main() -> i64 {
  block ^entry:
    %0 = load ptr @fp
    %1 = call i64 %0() as () -> i64
    return i64 %1
}
