function @store80(%value : f80) -> void {
 slot $local : f80
 block ^entry:
  store volatile f80 %value, $local
  return void
}
function @store32(%value : f64) -> void {
 slot $local : f32
 block ^entry:
  store volatile f32 %value, $local
  return void
}
function @store64(%value : f64) -> void {
 slot $local : f64
 block ^entry:
  store volatile f64 %value, $local
  return void
}
function @main() -> i64 [role=entry] {
 block ^entry:
  return i64 0
}
