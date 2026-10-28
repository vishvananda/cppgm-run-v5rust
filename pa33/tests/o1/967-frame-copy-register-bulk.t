global @source = { i64 11 i64 29 }
global @target = { i64 7 }
function @probe(%a : i64, %b : i64, %c : i64, %d : i64, %source : ptr, %target : ptr) -> i64 {
  slot $copy : obj<8x8>
  block ^entry:
    %second = index i64 %source, 1
    copyobj 8x8 %second, $copy
    zeroinit 8x8 %target
    %value = load i64 $copy
    return i64 %value
}
function @main() -> i32 [role=entry] {
  block ^entry:
    %value = call i64 @probe(0, 0, 0, 0, @source, @target)
    %bad = cmp ne i64 %value, 29
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}
