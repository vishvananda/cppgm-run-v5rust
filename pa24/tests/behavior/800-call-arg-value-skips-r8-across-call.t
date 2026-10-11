global @good : i64 = 0
global @finish : i64 = 0
global @tmp : i64 = 0
global @bad : i64 = 0

function @intervene(%scratch : ptr, %a : i64, %b : i64, %c : i64, %wrong : ptr) -> void {
  block ^entry:
    return void
}

function @relocate(%start : ptr, %finish : ptr, %tmp : ptr) -> void {
  block ^entry:
    store i64 77, %start
    store i64 88, %finish
    store i64 99, %tmp
    return void
}

function @run(%a : ptr, %b : ptr, %c : ptr) -> void {
  slot $a : ptr
  slot $b : ptr
  slot $c : ptr
  slot $scratch : obj<40x8>

  block ^entry:
    store ptr %a, $a
    store ptr %b, $b
    store ptr %c, $c
    jump ^loaded

  block ^loaded:
    %start = load ptr $a
    %finish = load ptr $b
    %temp = load ptr $c
    %scratch = addr $scratch
    %wrong = addr @bad
    call void @intervene(%scratch, 1, 2, 3, %wrong)
    call void @relocate(%start, %finish, %temp)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %good = addr @good
    %finish = addr @finish
    %tmp = addr @tmp
    call void @run(%good, %finish, %tmp)
    %good_value = load i64 @good
    %finish_value = load i64 @finish
    %tmp_value = load i64 @tmp
    %bad_value = load i64 @bad
    %good_ok = cmp eq i64 %good_value, 77
    %finish_ok = cmp eq i64 %finish_value, 88
    %tmp_ok = cmp eq i64 %tmp_value, 99
    %bad_ok = cmp eq i64 %bad_value, 0
    %ok1 = binary and i64 %good_ok, %finish_ok
    %ok2 = binary and i64 %tmp_ok, %bad_ok
    %ok = binary and i64 %ok1, %ok2
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
