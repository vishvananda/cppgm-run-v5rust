global @buffer = {
  i64 -1
  i64 -1
  i64 -1
  i64 -1
}

global @buffer_end : ptr = 0

function @fill_from_reference(%start : ptr, %n : i64, %value : ptr) -> void {
  block ^entry:
    %new_end = index i8 %start, %n
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %start, ^body: %next]
    %more = cmp ne ptr %p, %new_end
    branch %more, ^body, ^done

  block ^body:
    %v = load u8 %value
    store u8 %v, %p
    %next = index i8 %p, 1
    jump ^cond

  block ^done:
    store ptr %p, @buffer_end
    return void
}

function @zero_words(%start : ptr, %n : i64) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 2
    %last = index i8 %start, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %start, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store u32 0, %p
    %next = index i8 %p, 4
    jump ^cond

  block ^done:
    return void
}

function @check_bytes(%start : ptr, %n : i64, %expected : u8) -> i64 {
  block ^entry:
    jump ^cond

  block ^cond:
    %i = phi i64 [^entry: 0, ^next: %i1]
    %more = cmp lt i64 %i, %n
    branch %more, ^body, ^ok

  block ^body:
    %at = index i8 %start, %i
    %byte = load u8 %at
    %bad = cmp ne u8 %byte, %expected
    branch %bad, ^fail, ^next

  block ^next:
    %i1 = binary add i64 %i, 1
    jump ^cond

  block ^ok:
    return i64 0

  block ^fail:
    return i64 1
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  slot $seven : u8

  block ^entry:
    %base = addr @buffer
    store u8 7, $seven
    %seven_address = addr $seven
    call void @fill_from_reference(%base, 20, %seven_address)
    %filled = call i64 @check_bytes(%base, 20, 7)
    %tail = index i8 %base, 20
    %untouched = call i64 @check_bytes(%tail, 12, 255)
    %end = load ptr @buffer_end
    %end_expected = index i8 %base, 20
    %end_wrong = cmp ne ptr %end, %end_expected
    %end_bad = convert zext i64 i1 %end_wrong
    %inside = index i8 %base, 5
    store u8 3, %inside
    call void @fill_from_reference(%base, 10, %inside)
    %aliased = call i64 @check_bytes(%base, 10, 3)
    %rest = index i8 %base, 10
    %rest_kept = call i64 @check_bytes(%rest, 10, 7)
    call void @fill_from_reference(%base, 0, %seven_address)
    %zero_kept = call i64 @check_bytes(%base, 10, 3)
    call void @zero_words(%base, 3)
    %zeroed = call i64 @check_bytes(%base, 12, 0)
    %after_zero = index i8 %base, 12
    %after_kept = call i64 @check_bytes(%after_zero, 8, 7)
    %s1 = binary add i64 %filled, %untouched
    %s2 = binary add i64 %s1, %end_bad
    %s3 = binary add i64 %s2, %aliased
    %s4 = binary add i64 %s3, %rest_kept
    %s5 = binary add i64 %s4, %zero_kept
    %s6 = binary add i64 %s5, %zeroed
    %s7 = binary add i64 %s6, %after_kept
    %exit = convert trunc i32 i64 %s7
    return i32 %exit
}
