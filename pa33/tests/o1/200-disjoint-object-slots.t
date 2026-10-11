# Two mutually exclusive 1600-byte objects need one object area plus frame
# bookkeeping. Check MIR frame size, not a language-level process stack budget.
function @observe(%object : ptr) -> i32 [no_inline=yes] {
  block ^entry:
    %value = load i32 %object
    return i32 %value
}

function @probe(%choice : i32) -> i32 [no_inline=yes] {
  slot $left : obj<1600x4>
  slot $right : obj<1600x4>
  block ^entry:
    branch %choice, ^left, ^right
  block ^left:
    %left_address = addr $left
    store i32 7, %left_address
    %left_value = call i32 @observe(%left_address)
    return i32 %left_value
  block ^right:
    %right_address = addr $right
    store i32 9, %right_address
    %right_value = call i32 @observe(%right_address)
    return i32 %right_value
}

# Both branch objects can be visited in different turns of this loop. The
# saved pointer must still observe the first object after writing the second.
function @loop_overlap() -> i32 [no_inline=yes] {
  slot $first : obj<1600x4>
  slot $second : obj<1600x4>
  slot $saved : ptr
  slot $iteration : i32
  block ^entry:
    store i32 0, $iteration
    jump ^dispatch
  block ^dispatch:
    %iteration = load i32 $iteration
    branch %iteration, ^second, ^first
  block ^first:
    %first_address = addr $first
    store i32 7, %first_address
    store ptr %first_address, $saved
    store i32 1, $iteration
    jump ^dispatch
  block ^second:
    %second_address = addr $second
    store i32 9, %second_address
    %saved_address = load ptr $saved
    %saved_value = load i32 %saved_address
    %bad = cmp ne i32 %saved_value, 7
    return i32 %bad
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %left = call i32 @probe(1)
    %right = call i32 @probe(0)
    %left_bad = cmp ne i32 %left, 7
    %right_bad = cmp ne i32 %right, 9
    %branch_bad = binary or i32 %left_bad, %right_bad
    %loop_bad = call i32 @loop_overlap()
    %bad = binary or i32 %branch_bad, %loop_bad
    return i32 %bad
}
