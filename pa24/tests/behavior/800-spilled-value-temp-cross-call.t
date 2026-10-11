global @good : i64 = 7
global @bad : i64 = 13

function @clobber(%a : i64, %b : i64, %c : i64, %d : i64, %wrong : ptr) -> void {
  block ^entry:
    return void
}

function @return_saved(%target : ptr, %a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %g : i64, %h : i64) -> ptr {
  slot $saved : ptr

  block ^entry:
    store ptr %target, $saved
    jump ^after

  block ^after:
    %saved = load ptr $saved
    %wrong = addr @bad
    call void @clobber(1, 2, 3, 4, %wrong)
    return ptr %saved
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %good = addr @good
    %result = call ptr @return_saved(%good, 1, 2, 3, 4, 5, 6, 7, 8)
    %same = cmp eq ptr %result, %good
    %value = load i64 %result
    %value_ok = cmp eq i64 %value, 7
    %ok = binary and i64 %same, %value_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
