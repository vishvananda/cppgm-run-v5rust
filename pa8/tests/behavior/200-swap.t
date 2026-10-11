function @main() -> i64 [role=entry] {
  slot $left : i64
  slot $right : i64
  block ^entry:
    store i64 7, $left
    store i64 -11, $right
    %left = addr $left
    %right = addr $right
    call void @swap_values(%left, %right)
    %a = load i64 $left
    %b = load i64 $right
    %bad_a = cmp ne i64 %a, -11
    %bad_b = cmp ne i64 %b, 7
    call void @swap_values(%left, %left)
    %same = load i64 $left
    %bad_same = cmp ne i64 %same, -11
    call void @swap_values(%right, %left)
    %again_a = load i64 $left
    %again_b = load i64 $right
    %bad_again_a = cmp ne i64 %again_a, 7
    %bad_again_b = cmp ne i64 %again_b, -11
    %bad_ab = binary or i1 %bad_a, %bad_b
    %bad_again = binary or i1 %bad_again_a, %bad_again_b
    %bad_pairs = binary or i1 %bad_ab, %bad_again
    %bad = binary or i1 %bad_pairs, %bad_same
    %status = convert zext i64 i1 %bad
    return i64 %status
}
