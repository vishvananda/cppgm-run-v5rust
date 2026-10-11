function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  slot $source : obj<8x8>
  slot $middle : obj<8x8>
  slot $destination : obj<8x8>

  block ^entry:
    %source_address = addr $source
    store i64 42, %source_address
    %middle_address = addr $middle
    copyobj 8x8 %source_address, %middle_address
    %destination_address = addr $destination
    copyobj 8x8 %middle_address, %destination_address
    %actual = load i64 %destination_address
    %failed = cmp ne i64 %actual, 42
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
