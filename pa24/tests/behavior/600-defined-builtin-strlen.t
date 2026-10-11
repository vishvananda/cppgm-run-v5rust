global @text = { i8 97 i8 98 i8 99 i8 0 }

function @length(%s : ptr) -> i64 [object=cppgm_builtin_strlen] {
  block ^entry:
    jump ^loop
  block ^loop:
    %i = phi i64 [^entry: 0, ^more: %next]
    %address = index i8 %s, %i
    %byte = load i8 %address
    %end = cmp eq i8 %byte, 0
    branch %end, ^done, ^more
  block ^more:
    %next = binary add i64 %i, 1
    jump ^loop
  block ^done:
    return i64 %i
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %text = addr @text
    %length = call i64 @length(%text)
    return i64 %length
}
