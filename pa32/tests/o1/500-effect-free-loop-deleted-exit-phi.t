function @choose(%buffer : ptr, %new_last : ptr, %take : i64) -> ptr {
  block ^entry:
    %end = load ptr %buffer
    branch %take, ^pre, ^other

  block ^pre:
    jump ^cond

  block ^cond:
    %p = phi ptr [^pre: %end, ^body: %prev]
    %more = cmp ne ptr %new_last, %p
    branch %more, ^body, ^done

  block ^body:
    %prev = index i8 %p, -1
    jump ^cond

  block ^other:
    jump ^done

  block ^done:
    %result = phi ptr [^cond: %new_last, ^other: %end]
    return ptr %result
}
