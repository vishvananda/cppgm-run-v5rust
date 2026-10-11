global @table = {
  ptr addr @ret3
}

function @ret3() -> i64 {
  block ^entry:
    %0 = const i64 3
    return i64 %0
}

function @main() -> i64 {
  block ^entry:
    %0 = load ptr @table
    %1 = call i64 %0() as () -> i64
    return i64 %1
}
