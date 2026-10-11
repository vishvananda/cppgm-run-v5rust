function @increment(%cell : ptr) -> void {
  slot $cell : ptr

  block ^entry:
    store ptr %cell, $cell
    %address = load ptr $cell
    %value = load i32 %address
    %next = binary add i32 %value, 1
    store i32 %next, %address
    return void
}

function @main() -> i32 [role=entry, binding=strong] {
  slot $counter : obj<8x4>

  block ^entry:
    %base = addr $counter
    %cell = index i32 [projection=field] %base, 0
    store i32 0, %cell
    jump ^loop_condition

  block ^loop_condition:
    %value = load i32 %cell
    %done = cmp ge i32 %value, 5
    branch %done, ^loop_end, ^loop_body

  block ^loop_body:
    call void @increment(%cell)
    jump ^loop_condition

  block ^loop_end:
    %base_again = addr $counter
    %cell_again = index i32 [projection=field] %base_again, 0
    %result = load i32 %cell_again
    %ok = cmp eq i32 %result, 5
    branch %ok, ^good, ^bad

  block ^good:
    return i32 0

  block ^bad:
    return i32 1
}
