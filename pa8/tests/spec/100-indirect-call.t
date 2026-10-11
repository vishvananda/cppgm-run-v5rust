function @g(%x : i64) -> i64 {
  block ^entry:
    %0 = const i64 1
    %1 = binary add i64 %x, %0
    return i64 %1
}

function @call(%fp : ptr) -> i64 {
  block ^entry:
    %0 = const i64 4
    %1 = call i64 %fp(%0) as (%arg0 : i64) -> i64
    return i64 %1
}

function @main() -> i64 {
  block ^entry:
    %0 = addr @g
    %1 = call i64 @call(%0)
    return i64 %1
}
