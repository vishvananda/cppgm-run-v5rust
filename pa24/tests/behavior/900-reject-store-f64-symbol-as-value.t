global @g : i64 = 7
function @probe() -> f64 {
  slot $slot : f64
  block ^entry:
    %unrelated = const i128 99
    store volatile f64 @g, $slot
    return f64 0
}
function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
