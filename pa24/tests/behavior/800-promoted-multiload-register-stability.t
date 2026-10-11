function @calc(%n : i64) -> i64 {
  slot $n : i64

  block ^entry:
    %base = binary add i64 %n, 0
    store i64 %base, $n
    %first = load i64 $n
    %plus = binary add i64 %first, 2
    %second = load i64 $n
    %sum = binary add i64 %plus, %second
    return i64 %sum
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %got = call i64 @calc(10)
    %ok = cmp eq i64 %got, 22
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}
