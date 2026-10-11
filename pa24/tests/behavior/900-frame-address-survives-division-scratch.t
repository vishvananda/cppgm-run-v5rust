function @main() -> i32 [role=entry] {
  slot $bytes : obj<8x1>

  block ^entry:
    %base = addr $bytes
    store i8 65, %base
    %dividend = const i64 200
    %quotient = binary udiv i64 %dividend, 100
    %next = index i8 [projection=array_element] %base, 1
    %narrow = convert trunc i8 i64 %quotient
    store i8 %narrow, %next
    %result = load i8 %next
    %wide = convert zext i32 i8 %result
    %ok = cmp eq i32 %wide, 2
    %bad = cmp eq i32 %ok, 0
    return i32 %bad
}
