global @cells = {
  i64 11
  i64 13
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @cells
    %next = index i64 %base, 1
    %same = cmp eq ptr %next, %next
    %different = cmp ne ptr %next, %base
    %sum = binary add i64 %same, %different
    return i64 %sum
}
