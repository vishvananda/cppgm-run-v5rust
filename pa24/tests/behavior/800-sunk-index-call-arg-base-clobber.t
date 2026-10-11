global @object = {
  i64 0
  i64 0
}

global @wrong = {
  i64 0
  i64 0
}

function @store_field(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %p : ptr) -> void {
  block ^entry:
    store i64 77, %p
    return void
}

function @run() -> void {
  slot $base : ptr

  block ^entry:
    %object_base = addr @object
    store ptr %object_base, $base
    %base = load ptr $base
    %field = index i64 %base, 1
    call void @store_field(1, 2, 3, 4, 5, %field)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %object_base = addr @object
    %wrong_base = addr @wrong
    call void @run()
    %object_field = index i64 %object_base, 1
    %object_value = load i64 %object_field
    %wrong_field = index i64 %wrong_base, 1
    %wrong_value = load i64 %wrong_field
    %object_ok = cmp eq i64 %object_value, 77
    %wrong_ok = cmp eq i64 %wrong_value, 0
    %ok = binary and i64 %object_ok, %wrong_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
