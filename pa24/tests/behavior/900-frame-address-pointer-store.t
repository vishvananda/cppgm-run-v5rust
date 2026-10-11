function @touch() -> void {
  block ^entry:
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  slot $backing : obj<16x8>
  slot $saved : ptr

  block ^entry:
    %base = addr $backing
    store i64 1, %base
    call void @touch()
    store ptr %base, $saved
    %actual = load ptr $saved
    %same = cmp eq ptr %actual, %base
    %failed = cmp eq i64 %same, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
