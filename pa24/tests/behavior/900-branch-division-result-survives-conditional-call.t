function @touch() -> void {
  block ^entry:
    return void
}

function @choose(%value : i64, %enter : i64) -> i64 {
  block ^entry:
    %one = const i64 1
    %two = const i64 2
    %three = const i64 3
    %four = const i64 4
    %five = const i64 5
    %take_compute = cmp ne i64 %enter, 0
    branch %take_compute, ^compute, ^sibling

  block ^compute:
    %raw = binary div i64 %value, 1
    %negative = cmp lt i64 %raw, 0
    branch %negative, ^call_path, ^merge

  block ^call_path:
    call void @touch()
    jump ^merge

  block ^merge:
    %a = binary add i64 %one, %two
    %b = binary add i64 %three, %four
    %c = binary add i64 %a, %b
    %discard = binary add i64 %c, %five
    return i64 %raw

  block ^sibling:
    %d = binary add i64 %one, %two
    %e = binary add i64 %three, %four
    %f = binary add i64 %d, %e
    %unused = binary add i64 %f, %five
    return i64 0
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %warm = call i64 @choose(-9, 1)
    %actual = call i64 @choose(7, 1)
    %failed = cmp ne i64 %actual, 7
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
