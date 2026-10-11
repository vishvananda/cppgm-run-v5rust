function @touch() -> void {
  block ^entry:
    return void
}

function @loop_value(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64) -> i64 {
  slot $iteration : i64
  slot $result : i64

  block ^entry:
    store i64 0, $iteration
    store i64 0, $result
    jump ^loop_condition

  block ^loop_condition:
    %current = load i64 $iteration
    %more = cmp lt i64 %current, 1
    branch %more, ^loop_body, ^loop_end

  block ^loop_body:
    %from_e = binary add i64 %e, 1
    %ab = binary add i64 %a, %b
    %bc = binary add i64 %b, %c
    %cd = binary add i64 %c, %d
    call void @touch()
    %left = binary add i64 %ab, %bc
    %right = binary add i64 %cd, %from_e
    %partial = binary add i64 %left, %right
    %with_a = binary add i64 %partial, %a
    %with_b = binary add i64 %with_a, %b
    %with_c = binary add i64 %with_b, %c
    %value = binary add i64 %with_c, %d
    store i64 %value, $result
    %old = load i64 $iteration
    %next = binary add i64 %old, 1
    store i64 %next, $iteration
    jump ^loop_condition

  block ^loop_end:
    %answer = load i64 $result
    return i64 %answer
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %actual = call i64 @loop_value(1, 2, 3, 4, 5)
    %failed = cmp ne i64 %actual, 31
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
