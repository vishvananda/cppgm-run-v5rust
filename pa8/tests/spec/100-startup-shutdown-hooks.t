global @g : i64 = 0

function @boot() -> void [role=init] {
  block ^entry:
    %0 = const i64 5
    store i64 %0, @g
    return void
}

function @shutdown() -> void [role=fini] {
  block ^entry:
    %0 = const i64 9
    store i64 %0, @g
    return void
}

function @user_entry() -> i64 [role=entry] {
  block ^entry:
    %0 = load i64 @g
    return i64 %0
}
