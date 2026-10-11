function @encode(%this : ptr, %mode : u32, %reg : u32, %rm : u32) -> u32 {
  block ^entry:
    %mode_bits = binary shl u32 %mode, 6
    %reg_low = binary and u32 %reg, 7
    %reg_bits = binary shl u32 %reg_low, 3
    %prefix = binary or u32 %mode_bits, %reg_bits
    %rm_low = binary and u32 %rm, 7
    %result = binary or u32 %prefix, %rm_low
    return u32 %result
}

function @main() -> i32 [role=entry] {
  block ^entry:
    %actual = call u32 @encode(0, 3, 2, 5)
    %failed = cmp ne u32 %actual, 213
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
