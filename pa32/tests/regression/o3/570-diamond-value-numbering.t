declare function @observe(%value : i64) -> void [unwind=no]

function @repeated_size(%text : ptr) -> i64 [binding=strong] {
  block ^entry:
    %flag = load u8 %text
    %long = binary and u8 %flag, 1
    %is_long = cmp ne u8 %long, 0
    branch %is_long, ^first_long, ^first_short

  block ^first_long:
    %size_slot = index i8 [projection=field] %text, 8
    %long_size = load i64 %size_slot
    jump ^first_end

  block ^first_short:
    %shifted = binary ushr u8 %flag, 1
    %short_size = convert zext i64 u8 %shifted
    jump ^first_end

  block ^first_end:
    %size = phi i64 [^first_long: %long_size, ^first_short: %short_size]
    %doubled = binary add i64 %size, %size
    %again = copy ptr %text
    %flag2 = load u8 %again
    %long2 = binary and u8 %flag2, 1
    %is_long2 = cmp ne u8 %long2, 0
    branch %is_long2, ^second_long, ^second_short

  block ^second_long:
    %size_slot2 = index i8 [projection=field] %text, 8
    %long_size2 = load i64 %size_slot2
    jump ^second_end

  block ^second_short:
    %shifted2 = binary ushr u8 %flag2, 1
    %short_size2 = convert zext i64 u8 %shifted2
    jump ^second_end

  block ^second_end:
    %size2 = phi i64 [^second_long: %long_size2, ^second_short: %short_size2]
    %sum = binary add i64 %doubled, %size2
    call void @observe(%sum)
    %flag3 = load u8 %text
    %long3 = binary and u8 %flag3, 1
    %is_long3 = cmp ne u8 %long3, 0
    branch %is_long3, ^third_long, ^third_short

  block ^third_long:
    %data_slot = index i8 [projection=field] %text, 16
    %long_data = load ptr %data_slot
    jump ^third_end

  block ^third_short:
    %short_data = index i8 [projection=field] %text, 1
    jump ^third_end

  block ^third_end:
    %data = phi ptr [^third_long: %long_data, ^third_short: %short_data]
    %first = load u8 %data
    %widened = convert zext i64 u8 %first
    %total = binary add i64 %sum, %widened
    return i64 %total
}

function @size_after_store(%text : ptr, %other : ptr) -> i64 [binding=strong] {
  block ^entry:
    %flag = load u8 %text
    %long = binary and u8 %flag, 1
    %is_long = cmp ne u8 %long, 0
    branch %is_long, ^first_long, ^first_short

  block ^first_long:
    %size_slot = index i8 [projection=field] %text, 8
    %long_size = load i64 %size_slot
    jump ^first_end

  block ^first_short:
    %shifted = binary ushr u8 %flag, 1
    %short_size = convert zext i64 u8 %shifted
    jump ^first_end

  block ^first_end:
    %size = phi i64 [^first_long: %long_size, ^first_short: %short_size]
    store u8 0, %other
    %flag2 = load u8 %text
    %long2 = binary and u8 %flag2, 1
    %is_long2 = cmp ne u8 %long2, 0
    branch %is_long2, ^second_long, ^second_short

  block ^second_long:
    %size_slot2 = index i8 [projection=field] %text, 8
    %long_size2 = load i64 %size_slot2
    jump ^second_end

  block ^second_short:
    %shifted2 = binary ushr u8 %flag2, 1
    %short_size2 = convert zext i64 u8 %shifted2
    jump ^second_end

  block ^second_end:
    %size2 = phi i64 [^second_long: %long_size2, ^second_short: %short_size2]
    %sum = binary add i64 %size, %size2
    return i64 %sum
}
