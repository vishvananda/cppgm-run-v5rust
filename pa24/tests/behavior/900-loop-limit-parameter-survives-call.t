function @overwrite_argument_registers(%first : i64, %second : i64) -> i64 {
  block ^entry:
    %sum = binary add i64 %first, %second
    return i64 %sum
}

function @count_to(%initial : i64, %limit : i64) -> i64 {
  slot $current : i64

  block ^entry:
    store i64 %initial, $current
    jump ^loop_condition

  block ^loop_condition:
    %value = load i64 $current
    %more = cmp lt i64 %value, %limit
    branch %more, ^loop_body, ^loop_end

  block ^loop_body:
    %ignored = call i64 @overwrite_argument_registers(111, 222)
    %old = load i64 $current
    %next = binary add i64 %old, 1
    store i64 %next, $current
    jump ^loop_condition

  block ^loop_end:
    %result = load i64 $current
    return i64 %result
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %actual = call i64 @count_to(0, 3)
    %failed = cmp ne i64 %actual, 3
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
