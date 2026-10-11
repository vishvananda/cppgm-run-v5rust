global @out = {
  i64 0
  i64 0
}

function @make_pair(%ret : ptr, %a : i64, %b : i64) -> void {
  block ^entry:
    store i64 %a, %ret
    %0 = index i64 %ret, 1
    store i64 %b, %0
    return void
}

function @main() -> i64 {
  block ^entry:
    %0 = addr @out
    call void @make_pair(%0, 4, 5)
    %1 = load i64 @out
    %2 = index i64 %0, 1
    %3 = load i64 %2
    %4 = binary add i64 %1, %3
    return i64 %4
}
