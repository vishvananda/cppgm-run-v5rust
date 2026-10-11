global @buffer = {
  i64 -1
  i64 -1
  i64 -1
  i64 -1
  i64 -1
  i64 -1
}

function @fill_words(%first : ptr, %n : i64, %value : u32) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 2
    %last = index i8 %first, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store u32 %value, %p
    %next = index i8 %p, 4
    jump ^cond

  block ^done:
    return void
}

function @fill_halves(%first : ptr, %n : i64, %value : u16) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 1
    %last = index i8 %first, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store u16 %value, %p
    %next = index i8 %p, 2
    jump ^cond

  block ^done:
    return void
}

function @fill_quads(%first : ptr, %n : i64, %value : i64) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 3
    %last = index i8 %first, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store i64 %value, %p
    %next = index i8 %p, 8
    jump ^cond

  block ^done:
    return void
}

function @pattern_words(%first : ptr, %n : i64) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 2
    %last = index i8 %first, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    store u32 258, %p
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

function @check_words(%start : ptr, %n : i64, %expected : u32) -> i64 {
  block ^entry:
    jump ^cond

  block ^cond:
    %i = phi i64 [^entry: 0, ^next: %i1]
    %more = cmp lt i64 %i, %n
    branch %more, ^body, ^ok

  block ^body:
    %offset = binary shl i64 %i, 2
    %at = index i8 %start, %offset
    %word = load u32 %at
    %bad = cmp ne u32 %word, %expected
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
  block ^entry:
    %base = addr @buffer
    call void @fill_words(%base, 5, 305419896)
    %words = call i64 @check_words(%base, 5, 305419896)
    %tail = index i8 %base, 20
    %tail_kept = call i64 @check_bytes(%tail, 28, 255)
    call void @fill_halves(%base, 6, 43981)
    %half_words = call i64 @check_words(%base, 3, 2882382797)
    %half_tail = index i8 %base, 12
    %half_rest = call i64 @check_words(%half_tail, 2, 305419896)
    call void @fill_quads(%base, 4, -2401053088876216593)
    %quad_words = call i64 @check_words(%base, 8, 3735928559)
    %quad_tail = index i8 %base, 32
    %quad_rest = call i64 @check_bytes(%quad_tail, 16, 255)
    call void @pattern_words(%base, 2)
    %pattern = call i64 @check_words(%base, 2, 258)
    %pattern_rest = call i64 @check_words(%tail, 3, 3735928559)
    call void @fill_words(%base, 0, 7)
    %zero_kept = call i64 @check_words(%base, 2, 258)
    %s1 = binary add i64 %words, %tail_kept
    %s2 = binary add i64 %s1, %half_words
    %s3 = binary add i64 %s2, %half_rest
    %s4 = binary add i64 %s3, %quad_words
    %s5 = binary add i64 %s4, %quad_rest
    %s6 = binary add i64 %s5, %pattern
    %s7 = binary add i64 %s6, %pattern_rest
    %s8 = binary add i64 %s7, %zero_kept
    %exit = convert trunc i32 i64 %s8
    return i32 %exit
}
