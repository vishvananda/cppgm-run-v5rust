global @copied = {
  i64 0
}

function @touch() -> void {
  block ^entry:
    return void
}

function @make_tag() -> obj<8x8> {
  slot $tmp : obj<8x8>

  block ^entry:
    %p = addr $tmp
    store i64 77, %p
    return obj<8x8> $tmp
}

function @copy_after_call(%tag : obj<8x8>) -> void {
  block ^entry:
    call void @touch()
    %dst = addr @copied
    copyobj 8x8 %tag, %dst
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %tag = call obj<8x8> @make_tag()
    call void @copy_after_call(%tag)
    %v = load i64 @copied
    %ok = cmp eq i64 %v, 77
    %fail = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %fail
    return i32 %exit
}
