function @write_seventh(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %created : ptr) -> void {
  block ^entry:
    store u8 1, %created
    return void
}

function @main() -> i32 [role=entry] {
  slot $created : u8

  block ^entry:
    store u8 0, $created
    %address = addr $created
    call void @write_seventh(1, 2, 3, 4, 5, 6, %address)
    %value = load u8 $created
    %failed = cmp ne i64 %value, 1
    return i32 %failed
}
