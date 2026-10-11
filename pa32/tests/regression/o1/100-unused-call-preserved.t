function @ping() -> i64 {
  block ^entry:
    return i64 7
}
function @main() -> i64 {
  block ^entry:
    %unused = call i64 @ping()
    return i64 0
}
