declare function @observe_object(%object : ptr) -> void [unwind=no]

function @late_split(%p : ptr, %n : i64) -> i64 {
  slot $it : obj<16x8>
  slot $ref : ptr

  block ^entry:
    %a = addr $it
    store ptr %p, %a
    %f8 = index i8 [projection=field] %a, 8
    store i64 %n, %f8
    store ptr %a, $ref
    %r = load ptr $ref
    %q = load ptr %r
    %g8 = index i8 [projection=field] %r, 8
    %m = load i64 %g8
    %v = load i64 %q
    %s = binary add i64 %v, %m
    return i64 %s
}

function @keep_escaping_object(%p : ptr, %n : i64) -> i64 {
  slot $it : obj<16x8>

  block ^entry:
    %a = addr $it
    store ptr %p, %a
    %f8 = index i8 [projection=field] %a, 8
    store i64 %n, %f8
    call void @observe_object(%a)
    %m = load i64 %f8
    return i64 %m
}
