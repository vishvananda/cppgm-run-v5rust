global @good : i64 = 42
global @sink : i64 = 0
global @object = {
  i64 0
  i64 0
}

function @read_first(%p : ptr, %a : i64, %b : i64, %c : i64) -> i64 {
  block ^entry:
    %v = load i64 %p
    return i64 %v
}

function @run(%p : ptr, %q : i64, %r : i64, %s : i64, %t : i64, %u : i64, %v : i64) -> i64 {
  slot $p : ptr
  slot $scratch : i64

  block ^entry:
    store ptr %p, $p
    store i64 5, $scratch
    jump ^call

  block ^call:
    %scratch = load i64 $scratch
    store i64 %scratch, @sink
    %arg = load ptr $p
    %base = addr @object
    %field = index i8 [projection=field] %base, 8
    store i64 123456789, %field
    %got = call i64 @read_first(%arg, 1, 2, 3)
    return i64 %got
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %p = addr @good
    %got = call i64 @run(%p, 1, 2, 3, 4, 5, 6)
    %good_ok = cmp eq i64 %got, 42
    %sink = load i64 @sink
    %sink_ok = cmp eq i64 %sink, 5
    %object = addr @object
    %field = index i8 [projection=field] %object, 8
    %stored = load i64 %field
    %store_ok = cmp eq i64 %stored, 123456789
    %ok1 = binary and i64 %good_ok, %sink_ok
    %ok = binary and i64 %ok1, %store_ok
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}
