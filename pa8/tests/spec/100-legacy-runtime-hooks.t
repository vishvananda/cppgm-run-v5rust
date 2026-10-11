global @g : i64 = 0

function @__cppgm_init() -> void {
  block ^entry:
    store i64 7, @g
    return void
}

function @__cppgm_fini() -> void {
  block ^entry:
    return void
}

function @main() -> i64 {
  block ^entry:
    %value = load i64 @g
    return i64 %value
}
