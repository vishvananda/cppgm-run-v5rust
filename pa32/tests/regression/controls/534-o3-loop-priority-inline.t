function @fail() -> void
    [binding=internal, return=noreturn, no_inline=yes] {
  block ^entry:
    unreachable
}

function @advance(%value : i64) -> i64 [unwind=no, no_inline=yes] {
  block ^entry:
    %next = binary add i64 %value, 1
    return i64 %next
}

function @preferred_body(%value : i64) -> i64
    [binding=internal, unwind=no, inline_hint=yes] {
  block ^entry:
    %bad = cmp eq i64 %value, -1
    branch %bad, ^failure, ^work

  block ^failure:
    call void @fail()
    return i64 0

  block ^work:
    %v0 = call i64 @advance(%value)
    %v1 = call i64 @advance(%v0)
    %v2 = call i64 @advance(%v1)
    %v3 = call i64 @advance(%v2)
    %v4 = call i64 @advance(%v3)
    %v5 = call i64 @advance(%v4)
    %v6 = call i64 @advance(%v5)
    %v7 = call i64 @advance(%v6)
    %v8 = call i64 @advance(%v7)
    %v9 = call i64 @advance(%v8)
    %v10 = call i64 @advance(%v9)
    %v11 = call i64 @advance(%v10)
    %v12 = call i64 @advance(%v11)
    %v13 = call i64 @advance(%v12)
    %v14 = call i64 @advance(%v13)
    %v15 = call i64 @advance(%v14)
    %v16 = call i64 @advance(%v15)
    %v17 = call i64 @advance(%v16)
    %v18 = call i64 @advance(%v17)
    %v19 = call i64 @advance(%v18)
    %v20 = call i64 @advance(%v19)
    %v21 = call i64 @advance(%v20)
    %v22 = call i64 @advance(%v21)
    %v23 = call i64 @advance(%v22)
    %v24 = call i64 @advance(%v23)
    %v25 = call i64 @advance(%v24)
    %v26 = call i64 @advance(%v25)
    %v27 = call i64 @advance(%v26)
    %v28 = call i64 @advance(%v27)
    %v29 = call i64 @advance(%v28)
    %v30 = call i64 @advance(%v29)
    %v31 = call i64 @advance(%v30)
    %v32 = call i64 @advance(%v31)
    %v33 = call i64 @advance(%v32)
    %v34 = call i64 @advance(%v33)
    %v35 = call i64 @advance(%v34)
    %v36 = call i64 @advance(%v35)
    %v37 = call i64 @advance(%v36)
    %v38 = call i64 @advance(%v37)
    %v39 = call i64 @advance(%v38)
    %v40 = call i64 @advance(%v39)
    %v41 = call i64 @advance(%v40)
    %v42 = call i64 @advance(%v41)
    %v43 = call i64 @advance(%v42)
    %v44 = call i64 @advance(%v43)
    %v45 = call i64 @advance(%v44)
    %v46 = call i64 @advance(%v45)
    %v47 = call i64 @advance(%v46)
    %v48 = call i64 @advance(%v47)
    %v49 = call i64 @advance(%v48)
    %v50 = call i64 @advance(%v49)
    %v51 = call i64 @advance(%v50)
    %v52 = call i64 @advance(%v51)
    %v53 = call i64 @advance(%v52)
    %v54 = call i64 @advance(%v53)
    %v55 = call i64 @advance(%v54)
    %v56 = call i64 @advance(%v55)
    %v57 = call i64 @advance(%v56)
    %v58 = call i64 @advance(%v57)
    %v59 = call i64 @advance(%v58)
    %v60 = call i64 @advance(%v59)
    %v61 = call i64 @advance(%v60)
    %v62 = call i64 @advance(%v61)
    %v63 = call i64 @advance(%v62)
    return i64 %v63
}

function @loop_first(%limit : i64) -> i64 [unwind=no, no_inline=yes] {
  block ^entry:
    %cold = call i64 @preferred_body(0)
    jump ^header

  block ^header:
    %index = phi i64 [^entry: 0, ^latch: %next_index]
    %sum = phi i64 [^entry: %cold, ^latch: %hot]
    %more = cmp lt i64 %index, %limit
    branch %more, ^body, ^exit

  block ^body:
    %hot = call i64 @preferred_body(%sum)
    jump ^latch

  block ^latch:
    %next_index = binary add i64 %index, 1
    jump ^header

  block ^exit:
    return i64 %sum
}

function @loop_second(%limit : i64) -> i64 [unwind=no, no_inline=yes] {
  block ^entry:
    %cold = call i64 @preferred_body(0)
    jump ^header

  block ^header:
    %index = phi i64 [^entry: 0, ^latch: %next_index]
    %sum = phi i64 [^entry: %cold, ^latch: %hot]
    %more = cmp lt i64 %index, %limit
    branch %more, ^body, ^exit

  block ^body:
    %hot = call i64 @preferred_body(%sum)
    jump ^latch

  block ^latch:
    %next_index = binary add i64 %index, 1
    jump ^header

  block ^exit:
    return i64 %sum
}

function @no_loop_pair() -> i64 [unwind=no, no_inline=yes] {
  block ^entry:
    %first = call i64 @preferred_body(0)
    %second = call i64 @preferred_body(%first)
    return i64 %second
}

function @two_loop_calls() -> i64 [unwind=no, no_inline=yes] {
  block ^entry:
    jump ^header

  block ^header:
    %index = phi i64 [^entry: 0, ^body: %next_index]
    %sum = phi i64 [^entry: 0, ^body: %second]
    %more = cmp lt i64 %index, 1
    branch %more, ^body, ^exit

  block ^body:
    %first = call i64 @preferred_body(%sum)
    %second = call i64 @preferred_body(%first)
    %next_index = binary add i64 %index, 1
    jump ^header

  block ^exit:
    return i64 %sum
}

function @main() -> i64 [role=entry, unwind=no] {
  block ^entry:
    %first = call i64 @loop_first(5)
    %second = call i64 @loop_second(5)
    %plain = call i64 @no_loop_pair()
    %looped = call i64 @two_loop_calls()
    %sum0 = binary add i64 %first, %second
    %sum1 = binary add i64 %sum0, %plain
    %sum2 = binary add i64 %sum1, %looped
    %bad = cmp ne i64 %sum2, 1024
  return i64 %bad
}
