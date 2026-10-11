global @pair = {
  i64 0
  i64 0
}

global @source = {
  i64 11
  i64 22
  i64 33
  i64 44
}

global @dest = {
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
}

function @mark_two(%first : ptr, %second : ptr) -> void {
  block ^entry:
    store i64 99, %first
    store i64 77, %second
    return void
}

function @copy_then_mark_two(%this : ptr) -> void {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    %base0 = load ptr $this
    %field0 = index i8 [projection=field] %base0, 0
    %base1 = load ptr $this
    %field1 = index i8 [projection=field] %base1, 8
    %src = addr @source
    %dst = addr @dest
    copyobj 24x8 %src, %dst
    call void @mark_two(%field0, %field1)
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %pair = addr @pair
    call void @copy_then_mark_two(%pair)
    %first = load i64 @pair
    %second_ptr = index i8 %pair, 8
    %second = load i64 %second_ptr
    %first_ok = cmp eq i64 %first, 99
    %second_ok = cmp eq i64 %second, 77
    %ok = binary and i64 %first_ok, %second_ok
    %fail = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %fail
    return i32 %exit
}
