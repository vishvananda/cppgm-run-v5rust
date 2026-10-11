function @hash_bytes(%text : ptr, %first : i64, %count : i64) -> i64 {
  block ^entry:
    %end = binary add i64 %first, %count
    jump ^cond

  block ^cond:
    %i = phi i64 [^entry: %first, ^join: %i1]
    %h = phi i64 [^entry: 1469598103934665603, ^join: %h1]
    %more = cmp ult i64 %i, %end
    branch %more, ^body, ^done

  block ^body:
    %flag = load u8 %text
    %long_bit = binary and u8 %flag, 1
    %is_long = cmp ne u8 %long_bit, 0
    branch %is_long, ^long, ^short

  block ^long:
    %long_slot = index i8 %text, 16
    %data_long = load ptr %long_slot
    jump ^join

  block ^short:
    %data_short = index i8 %text, 1
    jump ^join

  block ^join:
    %data = phi ptr [^long: %data_long, ^short: %data_short]
    %at = index i8 %data, %i
    %byte = load u8 %at
    %wide = convert zext i64 u8 %byte
    %mixed = binary xor i64 %h, %wide
    %h1 = binary mul i64 %mixed, 1099511628211
    %i1 = binary add i64 %i, 1
    jump ^cond

  block ^done:
    return i64 %h
}

function @sum_kept_with_store(%values : ptr, %n : i64, %out : ptr) -> i64 {
  block ^entry:
    jump ^cond

  block ^cond:
    %i = phi i64 [^entry: 0, ^body: %i1]
    %s = phi i64 [^entry: 0, ^body: %s1]
    %more = cmp ult i64 %i, %n
    branch %more, ^body, ^done

  block ^body:
    %base = load ptr %values
    %offset = binary shl i64 %i, 3
    %at = index i8 %base, %offset
    %v = load i64 %at
    %s1 = binary add i64 %s, %v
    store i64 %s1, %out
    %i1 = binary add i64 %i, 1
    jump ^cond

  block ^done:
    return i64 %s
}
