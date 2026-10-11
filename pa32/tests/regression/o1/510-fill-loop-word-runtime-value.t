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
