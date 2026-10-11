global @cell : i64 = 0

function @touch(%p : ptr) -> void {
  block ^entry:
    return void
}

function @cell_ptr() -> ptr {
  block ^entry:
    %p = addr @cell
    return ptr %p
}

function @victim(%p : ptr) -> void {
  slot $shadow : ptr

  block ^entry:
    store ptr %p, $shadow
    %first = load ptr $shadow
    call void @touch(%first)
    %second = load ptr $shadow
    %result = call ptr @cell_ptr()
    %one = const i64 1
    store i64 77, %result
    call void @touch(%second)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %seed = addr @cell
    call void @victim(%seed)
    %value = load i64 @cell
    %ok = cmp eq i64 %value, 77
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
