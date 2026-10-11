global @cell = {
  i64 41
}

function @touch(%p : ptr, %n : i64) -> i64 {
  block ^entry:
    return i64 %n
}

function @dispatch(%which : i64, %this : ptr, %n : i64) -> i64 {
  block ^entry:
    %one = const i64 1
    switch %which, ^miss, %one:^case

  block ^case:
    %a = call i64 @touch(%this, %n)
    %b = call i64 @touch(%this, %n)
    return i64 %b

  block ^miss:
    return i64 0
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %base = addr @cell
    %which = const i64 1
    %n = const i64 7
    %value = call i64 @dispatch(%which, %base, %n)
    %ok = cmp eq i64 %value, 7
    branch %ok, ^good, ^bad

  block ^good:
    return i64 0

  block ^bad:
    return i64 1
}
