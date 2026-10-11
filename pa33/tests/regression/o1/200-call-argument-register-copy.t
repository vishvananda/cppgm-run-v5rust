function @sink(%x : i64) -> void {
  block ^entry:
    return void
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %value = binary add i64 20, 22
    call void @sink(%value)
    return i64 0
}
