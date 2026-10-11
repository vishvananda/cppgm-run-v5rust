global @cells = {
  i64 11
  i64 13
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @cells
    %next = index i64 %base, 1
    %delta = binary sub ptr %next, %base
    %value = load i64 %next
    %ok_delta = cmp eq i64 %delta, 1
    %ok_value = cmp eq i64 %value, 13
    %ok = binary and i64 %ok_delta, %ok_value
    return i64 %ok
}
