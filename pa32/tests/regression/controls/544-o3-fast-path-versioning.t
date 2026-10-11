global @seen : i64 [binding=internal] = 0
global @written : i64 [binding=internal] = 0

function @observe(%value : i64) -> void
    [binding=internal, no_inline=yes, unwind=no] {
  block ^entry:
    store volatile i64 %value, @seen
    return void
}

function @versionable(%value : i64, %choose : i64) -> void
    [binding=internal, unwind=no] {
  block ^entry:
    %take_left = cmp ne i64 %choose, 0
    branch %take_left, ^left, ^right

  block ^left:
    %left_value = binary add i64 %value, 1
    jump ^join

  block ^right:
    %right_value = binary sub i64 %value, 1
    jump ^join

  block ^join:
    %selected = phi i64 [^left: %left_value, ^right: %right_value]
    jump ^check0

  block ^check0:
    %match0 = cmp eq i64 %selected, 0
    branch %match0, ^slow0, ^check1

  block ^check1:
    %match1 = cmp eq i64 %value, 1
    branch %match1, ^slow1, ^check2

  block ^check2:
    %match2 = cmp eq i64 %value, 2
    branch %match2, ^slow2, ^check3

  block ^check3:
    %match3 = cmp eq i64 %value, 3
    branch %match3, ^slow3, ^check4

  block ^check4:
    %match4 = cmp eq i64 %value, 4
    branch %match4, ^slow0, ^check5

  block ^check5:
    %match5 = cmp eq i64 %value, 5
    branch %match5, ^slow1, ^check6

  block ^check6:
    %match6 = cmp eq i64 %value, 6
    branch %match6, ^slow2, ^check7

  block ^check7:
    %match7 = cmp eq i64 %value, 7
    branch %match7, ^slow3, ^fast

  block ^fast:
    return void

  block ^slow0:
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    return void

  block ^slow1:
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    return void

  block ^slow2:
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    return void

  block ^slow3:
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    call void @observe(%value)
    return void
}

function @effect_before_bailout(%value : i64, %out : ptr) -> void
    [binding=internal, unwind=no] {
  block ^check0:
    store i64 %value, %out
    %unsafe_match0 = cmp eq i64 %value, 0
    branch %unsafe_match0, ^unsafe_slow0, ^unsafe_check1

  block ^unsafe_check1:
    %unsafe_match1 = cmp eq i64 %value, 1
    branch %unsafe_match1, ^unsafe_slow1, ^unsafe_check2

  block ^unsafe_check2:
    %unsafe_match2 = cmp eq i64 %value, 2
    branch %unsafe_match2, ^unsafe_slow2, ^unsafe_check3

  block ^unsafe_check3:
    %unsafe_match3 = cmp eq i64 %value, 3
    branch %unsafe_match3, ^unsafe_slow3, ^unsafe_check4

  block ^unsafe_check4:
    %unsafe_match4 = cmp eq i64 %value, 4
    branch %unsafe_match4, ^unsafe_slow0, ^unsafe_check5

  block ^unsafe_check5:
    %unsafe_match5 = cmp eq i64 %value, 5
    branch %unsafe_match5, ^unsafe_slow1, ^unsafe_check6

  block ^unsafe_check6:
    %unsafe_match6 = cmp eq i64 %value, 6
    branch %unsafe_match6, ^unsafe_slow2, ^unsafe_check7

  block ^unsafe_check7:
    %unsafe_match7 = cmp eq i64 %value, 7
    branch %unsafe_match7, ^unsafe_slow3, ^unsafe_fast

  block ^unsafe_fast:
    return void

  block ^unsafe_slow0:
    %unsafe_value0 = binary add i64 %value, 1
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    call void @observe(%unsafe_value0)
    return void

  block ^unsafe_slow1:
    %unsafe_value1 = binary add i64 %value, 1
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    call void @observe(%unsafe_value1)
    return void

  block ^unsafe_slow2:
    %unsafe_value2 = binary add i64 %value, 1
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    call void @observe(%unsafe_value2)
    return void

  block ^unsafe_slow3:
    %unsafe_value3 = binary add i64 %value, 1
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    call void @observe(%unsafe_value3)
    return void
}

function @run_versionable(%value : i64, %choose : i64) -> void
    [no_inline=yes, unwind=no] {
  block ^entry:
    call void @versionable(%value, %choose)
    return void
}

function @run_versionable_again(%value : i64, %choose : i64) -> void
    [no_inline=yes, unwind=no] {
  block ^entry:
    call void @versionable(%value, %choose)
    return void
}

function @run_effect(%value : i64, %out : ptr) -> void
    [no_inline=yes, unwind=no] {
  block ^entry:
    call void @effect_before_bailout(%value, %out)
    return void
}

function @run_effect_again(%value : i64, %out : ptr) -> void
    [no_inline=yes, unwind=no] {
  block ^entry:
    call void @effect_before_bailout(%value, %out)
    return void
}

function @main() -> i32 [role=entry, unwind=no] {
  block ^entry:
    store i64 0, @seen
    call void @run_versionable(100, 1)
    %fast_seen = load volatile i64 @seen
    %fast_bad = cmp ne i64 %fast_seen, 0
    call void @run_versionable_again(100, 0)
    call void @run_versionable(1, 1)
    %slow_seen = load volatile i64 @seen
    %slow_bad = cmp ne i64 %slow_seen, 1
    store i64 0, @seen
    %written_address = addr @written
    call void @run_effect(100, %written_address)
    %written_fast = load i64 @written
    %unsafe_fast_seen = load volatile i64 @seen
    %written_fast_bad = cmp ne i64 %written_fast, 100
    %unsafe_fast_seen_bad = cmp ne i64 %unsafe_fast_seen, 0
    call void @run_effect_again(2, %written_address)
    %written_slow = load i64 @written
    %unsafe_slow_seen = load volatile i64 @seen
    %written_slow_bad = cmp ne i64 %written_slow, 2
    %unsafe_slow_seen_bad = cmp ne i64 %unsafe_slow_seen, 3
    %bad0 = binary or i32 %fast_bad, %slow_bad
    %bad1 = binary or i32 %written_fast_bad, %unsafe_fast_seen_bad
    %bad2 = binary or i32 %written_slow_bad, %unsafe_slow_seen_bad
    %bad3 = binary or i32 %bad0, %bad1
    %bad = binary or i32 %bad3, %bad2
    return i32 %bad
}
