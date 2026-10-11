function @main() -> i64 [role=entry] {
  slot $a : i32
  slot $b : i32

  block ^entry:
    store i32 0, $a
    store i32 2, $b
    jump ^cond

  block ^cond:
    %pa = addr $a
    %x = load i32 $a
    %pb = addr $b
    %y = load i32 $b
    %c = cmp ne i64 %x, %y
    branch %c, ^body, ^end

  block ^body:
    %x2 = load i32 $a
    %x3 = binary add i32 %x2, 1
    store i32 %x3, $a
    jump ^cond

  block ^end:
    return i64 0
}
