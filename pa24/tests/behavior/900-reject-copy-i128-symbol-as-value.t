global @g : i64 = 7
function @probe() -> i128 {
  slot $slot : i128
  block ^entry:
    %unrelated = const i128 99
    %x = copy i128 @g
    return i128 %x
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
