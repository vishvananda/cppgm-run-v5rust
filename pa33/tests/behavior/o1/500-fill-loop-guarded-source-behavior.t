global @outside_buffer = {
  i64 578437695752307201
  i64 1157159078456920585
}

global @aligned_buffer = {
  i64 578437695752307201
  i64 1157159078456920585
}

global @misaligned_buffer = {
  i64 578437695752307201
  i64 1157159078456920585
}

function @fill_words(%first : ptr, %n : i64, %value : ptr) -> void {
  block ^entry:
    %bytes = binary shl i64 %n, 2
    %last = index i8 %first, %bytes
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    %v = load u32 %value
    store u32 %v, %p
    %next = index i8 %p, 4
    jump ^cond

  block ^done:
    return void
}

function @word_at(%start : ptr, %index : i64) -> u32 {
  block ^entry:
    %offset = binary shl i64 %index, 2
    %at = index i8 %start, %offset
    %word = load u32 %at
    return u32 %word
}

function @mismatch(%actual : u32, %expected : u32) -> i64 {
  block ^entry:
    %bad = cmp ne u32 %actual, %expected
    %count = convert zext i64 i1 %bad
    return i64 %count
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %outside = addr @outside_buffer
    %outside_source = index i8 %outside, 12
    call void @fill_words(%outside, 3, %outside_source)
    %o0 = call u32 @word_at(%outside, 0)
    %o1 = call u32 @word_at(%outside, 1)
    %o2 = call u32 @word_at(%outside, 2)
    %o3 = call u32 @word_at(%outside, 3)
    %e0 = call i64 @mismatch(%o0, 269422093)
    %e1 = call i64 @mismatch(%o1, 269422093)
    %e2 = call i64 @mismatch(%o2, 269422093)
    %e3 = call i64 @mismatch(%o3, 269422093)
    %aligned = addr @aligned_buffer
    %aligned_source = index i8 %aligned, 4
    call void @fill_words(%aligned, 3, %aligned_source)
    %a0 = call u32 @word_at(%aligned, 0)
    %a1 = call u32 @word_at(%aligned, 1)
    %a2 = call u32 @word_at(%aligned, 2)
    %a3 = call u32 @word_at(%aligned, 3)
    %e4 = call i64 @mismatch(%a0, 134678021)
    %e5 = call i64 @mismatch(%a1, 134678021)
    %e6 = call i64 @mismatch(%a2, 134678021)
    %e7 = call i64 @mismatch(%a3, 269422093)
    %misaligned = addr @misaligned_buffer
    %misaligned_source = index i8 %misaligned, 2
    call void @fill_words(%misaligned, 3, %misaligned_source)
    %m0 = call u32 @word_at(%misaligned, 0)
    %m1 = call u32 @word_at(%misaligned, 1)
    %m2 = call u32 @word_at(%misaligned, 2)
    %m3 = call u32 @word_at(%misaligned, 3)
    %e8 = call i64 @mismatch(%m0, 100992003)
    %e9 = call i64 @mismatch(%m1, 100992517)
    %e10 = call i64 @mismatch(%m2, 100992517)
    %e11 = call i64 @mismatch(%m3, 269422093)
    %s1 = binary add i64 %e0, %e1
    %s2 = binary add i64 %s1, %e2
    %s3 = binary add i64 %s2, %e3
    %s4 = binary add i64 %s3, %e4
    %s5 = binary add i64 %s4, %e5
    %s6 = binary add i64 %s5, %e6
    %s7 = binary add i64 %s6, %e7
    %s8 = binary add i64 %s7, %e8
    %s9 = binary add i64 %s8, %e9
    %s10 = binary add i64 %s9, %e10
    %s11 = binary add i64 %s10, %e11
    %exit = convert trunc i32 i64 %s11
    return i32 %exit
}
