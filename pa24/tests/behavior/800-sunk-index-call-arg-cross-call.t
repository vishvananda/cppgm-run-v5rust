global @object = {
  i64 0
  i64 0
}

global @wrong = {
  i64 0
  i64 0
}

function @make(%a : i64, %b : i64, %c : i64, %d : i64, %wrong_base : ptr) -> i64 {
  block ^entry:
    return i64 42
}

function @store_value(%p : ptr, %value : i64) -> void {
  block ^entry:
    store i64 %value, %p
    return void
}

function @run() -> void {
  block ^entry:
    %base = addr @object
    %field = index i64 %base, 1
    %wrong_base = addr @wrong
    %value = call i64 @make(1, 2, 3, 4, %wrong_base)
    call void @store_value(%field, %value)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    call void @run()
    %object_base = addr @object
    %object_field = index i64 %object_base, 1
    %object_value = load i64 %object_field
    %wrong_base = addr @wrong
    %wrong_field = index i64 %wrong_base, 1
    %wrong_value = load i64 %wrong_field
    %object_ok = cmp eq i64 %object_value, 42
    %wrong_ok = cmp eq i64 %wrong_value, 0
    %ok = binary and i64 %object_ok, %wrong_ok
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
