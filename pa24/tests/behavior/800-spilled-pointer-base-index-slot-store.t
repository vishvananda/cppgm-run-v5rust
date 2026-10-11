global @buffer = {
  i64 10
  i64 20
  i64 30
  i64 40
}

global @which : i64 = 2

function @touch(%scratch : ptr) -> void {
  block ^entry:
    return void
}

function @run(%base : ptr) -> i64 {
  slot $saved : ptr
  slot $scratch : obj<40x8>

  block ^entry:
    %scratch = addr $scratch
    call void @touch(%scratch)
    %which = load i64 @which
    %end = index i64 %base, %which
    store ptr %end, $saved
    %saved = load ptr $saved
    %expected = index i64 %base, 2
    %same = cmp eq ptr %saved, %expected
    branch %same, ^check_value, ^bad

  block ^check_value:
    %value = load i64 %saved
    %value_ok = cmp eq i64 %value, 30
    %bad_value = cmp eq i64 %value_ok, 0
    return i64 %bad_value

  block ^bad:
    return i64 1
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @buffer
    %r = call i64 @run(%base)
    return i64 %r
}
