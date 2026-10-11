function @dynamic_four(%seed : i64) -> i64 {
  block ^entry:
    %value = binary add i64 %seed, 3
    return i64 %value
}

function @select_smaller(%seed : i64) -> i64 {
  slot $left_value : i64
  slot $right_value : i64
  slot $selected_address : ptr

  block ^entry:
    store i64 8, $left_value
    %left = addr $left_value
    %dynamic = call i64 @dynamic_four(%seed)
    store i64 %dynamic, $right_value
    %right = addr $right_value
    %right_loaded = load i64 %right
    %left_loaded = load i64 %left
    %right_is_smaller = cmp ult i64 %right_loaded, %left_loaded
    branch %right_is_smaller, ^select_right, ^select_left

  block ^select_right:
    store ptr %right, $selected_address
    jump ^selected

  block ^select_left:
    store ptr %left, $selected_address
    jump ^selected

  block ^selected:
    %address = load ptr $selected_address
    %value = load i64 %address
    return i64 %value
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call i64 @select_smaller(1)
    %failed = cmp ne i64 %actual, 4
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
