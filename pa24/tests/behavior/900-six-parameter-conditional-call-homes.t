function @touch() -> void {
  block ^entry:
    return void
}

function @select_width(%receiver : i64, %width : i64, %opcode : i64, %source : i64, %destination : i64, %scratch : i64) -> i64 {
  block ^entry:
    %is_sixteen = cmp eq i64 %width, 16
    branch %is_sixteen, ^prefix, ^continue

  block ^prefix:
    call void @touch()
    jump ^continue

  block ^continue:
    %receiver_width = binary add i64 %receiver, %width
    %opcode_source = binary add i64 %opcode, %source
    %destination_scratch = binary add i64 %destination, %scratch
    %left = binary add i64 %receiver_width, %opcode_source
    %result = binary add i64 %left, %destination_scratch
    return i64 %result
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call i64 @select_width(0, 64, 40, 1, 0, 3)
    %failed = cmp ne i64 %actual, 108
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
