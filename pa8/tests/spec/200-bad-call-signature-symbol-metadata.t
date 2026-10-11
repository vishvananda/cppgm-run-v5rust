function @callee() -> i64 {
  block ^entry:
    return i64 0
}

function @main() -> i64 {
  block ^entry:
    %fp = addr @callee
    %value = call i64 %fp() as () -> i64 [role=entry]
    return i64 %value
}
