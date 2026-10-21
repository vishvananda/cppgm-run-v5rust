function @copy_wide(%value : i128) -> i128 {
  slot $source : i128
  block ^entry:
    store volatile i128 %value, $source
    %copy = copy i128 $source
    return i128 %copy
}

function @main() -> i64 [role=entry] {
  slot $wide_source : i128
  slot $wide_target : i128
  slot $float_source : f80
  slot $float_target : f80
  slot $object_source : obj<8x8>
  slot $object_target : obj<8x8>
  block ^entry:
    %unused = const i128 99
    store volatile i128 18446744073709551657, $wide_source
    %wide = call i128 @copy_wide(18446744073709551657)
    store volatile i128 $wide_source, $wide_target
    %wide_again = load i128 $wide_target
    %bad_wide = cmp ne i128 %wide, 18446744073709551657
    %bad_again = cmp ne i128 %wide_again, 18446744073709551657
    store volatile f80 41.0, $float_source
    %float = copy f80 $float_source
    store volatile f80 $float_source, $float_target
    %float_again = load f80 $float_target
    %bad_float = cmp ne f80 %float, 41.0
    %bad_float_again = cmp ne f80 %float_again, 41.0
    store volatile i64 73, $object_source
    %object = copy obj<8x8> $object_source
    store volatile obj<8x8> %object, $object_target
    %object_value = load i64 $object_target
    %bad_object = cmp ne i64 %object_value, 73
    store volatile i64 97, $object_source
    store volatile obj<8x8> $object_source, $object_target
    %object_again = load i64 $object_target
    %bad_object_again = cmp ne i64 %object_again, 97
    %b0 = binary or i64 %bad_wide, %bad_again
    %b1 = binary or i64 %bad_float, %bad_float_again
    %b2 = binary or i64 %bad_object, %bad_object_again
    %b3 = binary or i64 %b0, %b1
    %failed = binary or i64 %b3, %b2
    return i64 %failed
}
