global @buffer = {
  i64 10
  i64 20
  i64 30
  i64 40
}

global @which : i64 = 2
global @saved : ptr = 0

function @touch(%scratch : ptr) -> void {
  block ^entry:
    return void
}

function @run() -> void {
  slot $scratch : obj<40x8>

  block ^entry:
    %scratch = addr $scratch
    call void @touch(%scratch)
    %base = addr @buffer
    %which = load i64 @which
    %end = index i64 %base, %which
    store ptr %end, @saved
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    call void @run()
    %base = addr @buffer
    %expected = index i64 %base, 2
    %saved = load ptr @saved
    %same = cmp eq ptr %saved, %expected
    %value = load i64 %saved
    %value_ok = cmp eq i64 %value, 30
    %ok = binary and i64 %same, %value_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
