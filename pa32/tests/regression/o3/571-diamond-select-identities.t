declare function @observe(%value : i64) -> void [unwind=no]

function @minimum_with_npos(%n : i64) -> i64 [binding=strong] {
  block ^entry:
    %below = cmp ult i64 %n, -1
    branch %below, ^take_n, ^take_npos

  block ^take_n:
    jump ^merge

  block ^take_npos:
    jump ^merge

  block ^merge:
    %least = phi i64 [^take_n: %n, ^take_npos: -1]
    return i64 %least
}

function @maximum_with_zero(%n : i64) -> i64 [binding=strong] {
  block ^entry:
    %above = cmp ugt i64 %n, 0
    branch %above, ^take_n, ^take_zero

  block ^take_n:
    jump ^merge

  block ^take_zero:
    jump ^merge

  block ^merge:
    %most = phi i64 [^take_n: %n, ^take_zero: 0]
    return i64 %most
}

function @equal_choice(%n : i64) -> i64 [binding=strong] {
  block ^entry:
    %same = cmp eq i64 %n, 7
    branch %same, ^take_seven, ^take_n

  block ^take_seven:
    jump ^merge

  block ^take_n:
    jump ^merge

  block ^merge:
    %chosen = phi i64 [^take_seven: 7, ^take_n: %n]
    return i64 %chosen
}

function @minimum_stays(%n : i64, %limit : i64) -> i64 [binding=strong] {
  block ^entry:
    %below = cmp ult i64 %n, %limit
    branch %below, ^take_n, ^take_limit

  block ^take_n:
    jump ^merge

  block ^take_limit:
    jump ^merge

  block ^merge:
    %least = phi i64 [^take_n: %n, ^take_limit: %limit]
    return i64 %least
}

function @length_compare(%size : i64) -> i64 [binding=strong] {
  block ^entry:
    %same = cmp ne i64 3, %size
    branch %same, ^differ, ^check

  block ^differ:
    return i64 0

  block ^check:
    %below = cmp ult i64 %size, -1
    branch %below, ^take_size, ^take_npos

  block ^take_size:
    jump ^bounded

  block ^take_npos:
    jump ^bounded

  block ^bounded:
    %length = phi i64 [^take_size: %size, ^take_npos: -1]
    %shorter = cmp ult i64 3, %length
    branch %shorter, ^take_three, ^take_length

  block ^take_three:
    jump ^count

  block ^take_length:
    jump ^count

  block ^count:
    %n = phi i64 [^take_three: 3, ^take_length: %length]
    call void @observe(%n)
    return i64 %n
}
