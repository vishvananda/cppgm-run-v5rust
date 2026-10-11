global @good : i64 = 42
global @wrong = {
  i64 13
  i64 99
}

function @capture(%p : ptr, %addr : ptr, %b : i64, %c : i64) -> i64 {
  block ^entry:
    %v = load i64 %p
    return i64 %v
}

function @run(%p : ptr, %q : i64, %r : i64, %s : i64, %t : i64, %u : i64, %v : i64) -> i64 {
  slot $p : ptr

  block ^entry:
    store ptr %p, $p
    jump ^call

  block ^call:
    %this = load ptr $p
    %base = addr @wrong
    %addr = index i64 %base, 1
    %got = call i64 @capture(%this, %addr, 2, 3)
    return i64 %got
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %p = addr @good
    %got = call i64 @run(%p, 2, 3, 4, 5, 6, 7)
    %ok = cmp eq i64 %got, 42
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}
