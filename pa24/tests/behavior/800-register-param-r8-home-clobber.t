global @value : i64 = 77

function @consume(%p : ptr) -> i64 {
  block ^entry:
    return i64 0
}

function @load_value(%p : ptr) -> i64 {
  block ^entry:
    %v = load i64 %p
    return i64 %v
}

function @target(%a : ptr, %b : ptr, %c : ptr, %d : ptr, %e : ptr) -> i64 {
  slot $b : ptr
  slot $e : ptr

  block ^entry:
    store ptr %b, $b
    store ptr %e, $e
    %ep = load ptr $e
    %result = call i64 @load_value(%ep)
    %bp = load ptr $b
    %ignored = call i64 @consume(%bp)
    return i64 %result
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %p = addr @value
    %r = call i64 @target(0, 0, 0, 0, %p)
    %ok = cmp eq i64 %r, 77
    %bad = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}
