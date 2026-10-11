function @sink(%v : i64) -> i64 {
  block ^entry:
    return i64 %v
}

function @run(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %g : i64) -> i64 {
  slot $cell : i64

  block ^entry:
    store i64 42, $cell
    %base = addr $cell
    %ptr = index i64 %base, 0
    %val = load i64 %ptr
    %got = call i64 @sink(%val)
    return i64 %got
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %got = call i64 @run(1, 2, 3, 4, 5, 6, 7)
    %ok = cmp eq i64 %got, 42
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}
