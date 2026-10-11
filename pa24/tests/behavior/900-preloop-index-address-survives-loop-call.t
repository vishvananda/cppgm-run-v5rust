function @return_one() -> i64 {
  block ^entry:
    return i64 1
}

function @loop_sum(%a : i64, %b : i64, %c : i64, %d : i64) -> i64 {
  slot $storage : obj<16x8>
  slot $current : i64

  block ^entry:
    %base = addr $storage
    %second = index i8 %base, 8
    store i64 7, %second
    store i64 0, $current
    jump ^loop_condition

  block ^loop_condition:
    %value = load i64 $current
    %more = cmp lt i64 %value, 3
    branch %more, ^loop_body, ^loop_end

  block ^loop_body:
    %one = call i64 @return_one()
    %valid = cmp eq i64 %one, 1
    branch %valid, ^loop_step, ^failure

  block ^loop_step:
    %old = load i64 $current
    %next = binary add i64 %old, 1
    store i64 %next, $current
    jump ^loop_condition

  block ^loop_end:
    %ab = binary add i64 %a, %b
    %cd = binary add i64 %c, %d
    %abcd = binary add i64 %ab, %cd
    %stored = load i64 %second
    %result = binary add i64 %abcd, %stored
    return i64 %result

  block ^failure:
    return i64 -1
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %actual = call i64 @loop_sum(10, 20, 30, 40)
    %failed = cmp ne i64 %actual, 107
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
