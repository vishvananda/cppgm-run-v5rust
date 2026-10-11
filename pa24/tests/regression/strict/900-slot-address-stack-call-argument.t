function @write_seventh(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %created : ptr) -> void {
  block ^entry:
    store u8 1, %created
    return void
}

function @check_bulk_scalar_uses() -> i32 {
  slot $buffer : obj<16x1>
  slot $copied : obj<16x1>

  block ^entry:
    %base = addr $buffer
    zeroinit 16x1 %base
    %end = index i8 %base, 10
    %forward = binary sub ptr %end, %base
    %reverse = binary sub ptr %base, %end
    %bad_forward = cmp ne i64 %forward, 10
    %bad_reverse = cmp ne i64 %reverse, -10
    %bad_distance = binary or i32 %bad_forward, %bad_reverse

    %copied = addr $copied
    copyobj 16x1 %base, %copied
    %same = addr $copied
    %bad_compare_left = cmp ne ptr %copied, %same
    %bad_compare_right = cmp ne ptr %same, %copied
    %bad_compare = binary or i32 %bad_compare_left, %bad_compare_right
    %failed = binary or i32 %bad_distance, %bad_compare
    return i32 %failed
}

function @main() -> i32 [role=entry] {
  slot $created : u8

  block ^entry:
    store u8 0, $created
    %address = addr $created
    call void @write_seventh(1, 2, 3, 4, 5, 6, %address)
    %value = load u8 $created
    %bad_seventh = cmp ne i64 %value, 1
    %bad_bulk = call i32 @check_bulk_scalar_uses()
    %failed = binary or i32 %bad_seventh, %bad_bulk
    return i32 %failed
}
