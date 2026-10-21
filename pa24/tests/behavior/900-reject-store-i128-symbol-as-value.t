global @g : i64 = 7
function @probe() -> i128 {
  slot $slot : i128
  block ^entry:
    %unrelated = const i128 99
    store volatile i128 @g, $slot
    return i128 0
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
