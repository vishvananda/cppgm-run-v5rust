global @cell = {
  i64 41
}

function @touch(%p : ptr, %n : i64) -> i64 {
  block ^entry:
    return i64 %n
}

function @load_value(%p : ptr) -> i64 {
  block ^entry:
    %value = load i64 %p
    return i64 %value
}

function @read_after_call(%this : ptr, %n : i64) -> i64 {
  block ^entry:
    %base = index i8 [projection=field] %this, 0
    %dead = call i64 @touch(%this, %n)
    %value = call i64 @load_value(%base)
    return i64 %value
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @cell
    %n = const i64 7
    %value = call i64 @read_after_call(%base, %n)
    %ok = cmp eq i64 %value, 41
    branch %ok, ^good, ^bad

  block ^good:
    return i64 0

  block ^bad:
    return i64 1
}
