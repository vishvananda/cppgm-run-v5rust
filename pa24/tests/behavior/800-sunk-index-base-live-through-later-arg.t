global @pair = {
  i64 0
  i64 0
}

function @store_dec(%p : ptr, %dec : i64) -> void {
  block ^entry:
    store i64 99, %p
    return void
}

function @call_field_with_later_arg(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    store ptr %this, $this
    %base = load ptr $this
    %field = index i8 [projection=field] %base, 8
    %dec = const i64 -1
    call void @store_dec(%field, %dec)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %pair = addr @pair
    call void @call_field_with_later_arg(%pair)
    %first = load i64 @pair
    %second_ptr = index i8 %pair, 8
    %second = load i64 %second_ptr
    %first_ok = cmp eq i64 %first, 0
    %second_ok = cmp eq i64 %second, 99
    %ok = binary and i64 %first_ok, %second_ok
    %fail = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %fail
    return i32 %exit
}
