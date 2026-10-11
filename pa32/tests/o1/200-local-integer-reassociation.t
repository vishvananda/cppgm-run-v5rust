function @add_chain(%x : i64) -> i64 {
  block ^entry:
    %a = binary add i64 %x, 3
    %b = binary add i64 %a, 4
    return i64 %b
}
function @mul_chain(%x : i64) -> i64 {
  block ^entry:
    %a = binary mul i64 %x, 5
    %b = binary mul i64 %a, 2
    return i64 %b
}
function @and_chain(%x : u32) -> u32 {
  block ^entry:
    %a = binary and u32 %x, 255
    %b = binary and u32 %a, 15
    return u32 %b
}
