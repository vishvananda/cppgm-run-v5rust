function @count(%first : ptr, %last : ptr) -> i64 {
  block ^entry:
    jump ^cond

  block ^cond:
    %p = phi ptr [^entry: %first, ^body: %next]
    %n = phi i64 [^entry: 0, ^body: %n1]
    %more = cmp ne ptr %p, %last
    branch %more, ^body, ^done

  block ^body:
    %next = index i8 %p, 1
    %n1 = binary add i64 %n, 1
    jump ^cond

  block ^done:
    return i64 %n
}
