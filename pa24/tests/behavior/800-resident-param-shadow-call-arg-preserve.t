global @good : i64 = 0
global @bad : i64 = 0

function @clobber(%temp : ptr, %a : i64, %b : i64, %c : i64, %wrong : ptr) -> void {
  block ^entry:
    return void
}

function @assign(%lhs : ptr, %rhs : ptr) -> void {
  block ^entry:
    store i64 99, %lhs
    return void
}

function @reset(%target : ptr) -> void {
  slot $target : ptr
  slot $temp : obj<40x8>

  block ^entry:
    store ptr %target, $target
    %saved = load ptr $target
    %temp = addr $temp
    %wrong = addr @bad
    call void @clobber(%temp, 1, 2, 3, %wrong)
    call void @assign(%saved, %temp)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %target = addr @good
    call void @reset(%target)
    %good_value = load i64 @good
    %bad_value = load i64 @bad
    %good_ok = cmp eq i64 %good_value, 99
    %bad_ok = cmp eq i64 %bad_value, 0
    %ok = binary and i64 %good_ok, %bad_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
