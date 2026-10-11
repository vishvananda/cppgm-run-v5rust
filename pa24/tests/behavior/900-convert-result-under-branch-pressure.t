function @pressure(%width : u32, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %gate : i64) -> i64 {
  slot $selected : i64

  block ^entry:
    branch %gate, ^convert, ^short

  block ^convert:
    %kept = binary add i64 %b, 0
    %extended = convert zext i64 u32 %width
    %shifted = binary shl i64 1, %extended
    store i64 %shifted, $selected
    jump ^merge

  block ^short:
    store i64 0, $selected
    jump ^merge

  block ^merge:
    %picked = load i64 $selected
    %bc = binary add i64 %b, %c
    %de = binary add i64 %d, %e
    %left = binary add i64 %bc, %de
    %right = binary add i64 %f, %kept
    %all = binary add i64 %left, %right
    %width64 = convert zext i64 u32 %width
    %with_width = binary add i64 %all, %width64
    %result = binary add i64 %with_width, %picked
    return i64 %result
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call i64 @pressure(6, 2, 3, 4, 5, 6, 1)
    %failed = cmp ne i64 %actual, 92
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
