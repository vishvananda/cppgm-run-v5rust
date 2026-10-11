global @g : i64 = 0

function @__cppgm_init() -> void [role=init] {
  block ^entry:
    %0 = const i64 5
    store i64 %0, @g
    return void
}

function @__cppgm_fini() -> void [role=fini] {
  block ^entry:
    %0 = const i64 9
    store i64 %0, @g
    return void
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = load i64 @g
    return i64 %0
}
