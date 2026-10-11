global @byte : u8 = 42

function @touch() -> void {
  block ^entry:
    return void
}

function @source(%value : i64) -> i64 {
  block ^entry:
    return i64 %value
}

function @pressure(%high : i64, %enter : i64, %offset : i64, %a : i64, %b : i64, %c : i64) -> i64 {
  block ^entry:
    %take_high = cmp ne i64 %high, 0
    branch %take_high, ^high_pressure, ^select

  block ^select:
    %take_compute = cmp ne i64 %enter, 0
    branch %take_compute, ^compute, ^sibling_setup

  block ^sibling_setup:
    %sib_ab = binary add i64 %a, %b
    %sib_bc = binary add i64 %b, %c
    jump ^sibling_use

  block ^compute:
    %base = addr @byte
    %dynamic_offset = binary add i64 %offset, 0
    %address = index i8 %base, %dynamic_offset
    call void @touch()
    %loaded = load i8 %address
    %loaded64 = convert zext i64 i8 %loaded
    %ab = binary add i64 %a, %b
    %parameters = binary add i64 %ab, %c
    %result = binary add i64 %loaded64, %parameters
    return i64 %result

  block ^sibling_use:
    %sib_left = binary add i64 %sib_ab, %a
    %sib_right = binary add i64 %sib_bc, %c
    %sib_result = binary add i64 %sib_left, %sib_right
    return i64 %sib_result

  block ^high_pressure:
    %q1 = call i64 @source(1)
    %q2 = call i64 @source(2)
    %q3 = call i64 @source(3)
    %q4 = call i64 @source(4)
    %q5 = call i64 @source(5)
    %q6 = call i64 @source(6)
    %q7 = call i64 @source(7)
    %q8 = call i64 @source(8)
    call void @touch()
    %q12 = binary add i64 %q1, %q2
    %q34 = binary add i64 %q3, %q4
    %q56 = binary add i64 %q5, %q6
    %q78 = binary add i64 %q7, %q8
    %q1234 = binary add i64 %q12, %q34
    %q5678 = binary add i64 %q56, %q78
    %high_result = binary add i64 %q1234, %q5678
    return i64 %high_result
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call i64 @pressure(0, 1, 0, 1, 2, 3)
    %failed = cmp ne i64 %actual, 48
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
