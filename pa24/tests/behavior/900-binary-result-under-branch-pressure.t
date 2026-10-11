function @pressure(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %gate : i64) -> i64 {
  slot $selected : i64

  block ^entry:
    branch %gate, ^shift, ^short

  block ^shift:
    %count = binary and i64 %a, 7
    %shifted = binary shl i64 1, %count
    store i64 %shifted, $selected
    jump ^merge

  block ^short:
    store i64 0, $selected
    jump ^merge

  block ^merge:
    %picked = load i64 $selected
    %ab = binary add i64 %a, %b
    %cd = binary add i64 %c, %d
    %ef = binary add i64 %e, %f
    %abcd = binary add i64 %ab, %cd
    %all = binary add i64 %abcd, %ef
    %result = binary add i64 %all, %picked
    return i64 %result
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call i64 @pressure(6, 2, 3, 4, 5, 6, 1)
    %failed = cmp ne i64 %actual, 90
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
