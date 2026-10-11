global @a : i64 = 0
global @b : i64 = 0

function @target(%x : ptr, %y : ptr) -> void {
  block ^entry:
    return void
}

function @run(%p : ptr, %q : ptr) -> i64 {
  slot $tiny : i8

  block ^entry:
    %live = const i64 7
    call void @target(%q, %p)
    store i8 1, $tiny
    return i64 %live
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %p = addr @a
    %q = addr @b
    %keep = const i64 1234605616436508552
    %got = call i64 @run(%p, %q)
    %ok_keep = cmp eq i64 %keep, 1234605616436508552
    %ok_got = cmp eq i64 %got, 7
    %ok = binary and i64 %ok_keep, %ok_got
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
