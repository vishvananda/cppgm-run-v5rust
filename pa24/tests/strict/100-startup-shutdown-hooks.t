global @g : i64 = 0
declare function @atexit(%callback : ptr) -> i32

function @__cppgm_init() -> void [role=init] {
  block ^entry:
    %0 = const i64 5
    store i64 %0, @g
    return void
}

function @__cppgm_fini() -> void [role=fini] {
  block ^entry:
    %value = load i64 @g
    %ok = cmp eq i64 %value, 8
    branch %ok, ^finish, ^fail
  block ^fail:
    store i64 1, 0
    return void
  block ^finish:
    %0 = const i64 9
    store i64 %0, @g
    return void
}

function @first() -> void {
  block ^entry:
    %value = load i64 @g
    %ok = cmp eq i64 %value, 7
    branch %ok, ^finish, ^fail
  block ^fail:
    store i64 1, 0
    return void
  block ^finish:
    store i64 8, @g
    return void
}

function @third() -> void {
  block ^entry:
    %value = load i64 @g
    %ok = cmp eq i64 %value, 6
    branch %ok, ^finish, ^fail
  block ^fail:
    store i64 1, 0
    return void
  block ^finish:
    store i64 7, @g
    return void
}

function @second() -> void {
  block ^entry:
    store i64 6, @g
    %third = addr @third
    %registered = call i32 @atexit(%third)
    return void
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %first = addr @first
    %registered_first = call i32 @atexit(%first)
    %second = addr @second
    %registered_second = call i32 @atexit(%second)
    %0 = load i64 @g
    return i64 %0
}
