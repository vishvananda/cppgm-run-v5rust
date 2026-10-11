global @object = {
  i64 42
  i64 0
  i64 0
  ptr addr @read_value
}

function @read_value(%tag : i64, %p : ptr) -> i64 {
  block ^entry:
    %v = load i64 %p
    %sum = binary add i64 %v, %tag
    return i64 %sum
}

function @invoke(%this : ptr) -> i64 {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    %base = load ptr $this
    %arg = index i8 [projection=field] %base, 0
    %fnslot = index i8 [projection=field] %base, 24
    %fp = load ptr %fnslot
    %r = call i64 %fp(5, %arg) as (%arg0 : i64, %arg1 : ptr) -> i64
    return i64 %r
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %p = addr @object
    %r = call i64 @invoke(%p)
    %bad = cmp ne i64 %r, 47
    return i64 %bad
}
