global @first : i64 = 11
global @second : i64 = 22

function @identity(%value : ptr) -> ptr {
  block ^entry:
    return ptr %value
}

function @check(%first : ptr, %value : f80, %second : ptr) -> i64 {
  block ^entry:
    %expected_first = addr @first
    %expected_second = addr @second
    %first_ok = cmp eq ptr %first, %expected_first
    %value_ok = cmp eq f80 %value, 3.5L
    %second_ok = cmp eq ptr %second, %expected_second
    %pointers_ok = binary and i64 %first_ok, %second_ok
    %all_ok = binary and i64 %pointers_ok, %value_ok
    return i64 %all_ok
}

function @forward(%first : ptr, %value : f80, %second : ptr) -> i64 {
  slot $first : ptr
  slot $value : f80
  slot $second : ptr

  block ^entry:
    store ptr %first, $first
    store f80 %value, $value
    store ptr %second, $second
    %stored_first = load ptr $first
    %loaded_first = call ptr @identity(%stored_first)
    %loaded_value = load f80 $value
    %loaded_second = load ptr $second
    %result = call i64 @check(%loaded_first, %loaded_value, %loaded_second)
    return i64 %result
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %first = addr @first
    %second = addr @second
    %result = call i64 @forward(%first, 3.5L, %second)
    return i64 %result
}
