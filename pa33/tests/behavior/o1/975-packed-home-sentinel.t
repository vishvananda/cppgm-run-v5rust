function @edge(%value : i64) -> i64 {
  slot $data : obj<1073741824x8>
  block ^entry:
    eh_try ^handler
    eh_end
    %address = index i8 $data, -1073741824
    store i64 %value, %address
    %loaded = load i64 %address
    %next = binary add i64 %loaded, 1
    return i64 %next
  block ^handler:
    return i64 99
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
