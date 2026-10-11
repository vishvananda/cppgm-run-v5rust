function @f() -> i64 {
  slot $x : i64

  block ^entry:
    %0 = const i64 2
    store i64 %0, $x
    jump ^cond

  block ^cond:
    %1 = load i64 $x
    %2 = const i64 0
    %3 = cmp gt i64 %1, %2
    branch %3, ^body, ^end

  block ^body:
    %4 = load i64 $x
    %5 = const i64 1
    %6 = binary sub i64 %4, %5
    store i64 %6, $x
    %7 = load i64 $x
    %8 = const i64 1
    %9 = cmp eq i64 %7, %8
    branch %9, ^end, ^cont

  block ^cont:
    jump ^cond

  block ^end:
    %10 = load i64 $x
    return i64 %10
}

function @main() -> i64 {
  block ^entry:
    %0 = call i64 @f()
    return i64 %0
}
