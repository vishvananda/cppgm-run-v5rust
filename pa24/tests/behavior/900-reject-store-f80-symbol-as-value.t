global @g : i64 = 7
function @probe() -> f80 {
  slot $slot : f80
  block ^entry:
    %unrelated = const i128 99
    store volatile f80 @g, $slot
    return f80 0
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
