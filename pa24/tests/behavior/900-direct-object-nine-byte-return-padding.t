global @copied = {
  zero 9
}

function @make() -> obj<9x1> {
  slot $value : obj<9x1>

  block ^entry:
    %base = addr $value
    zeroinit 9x1 %base
    store i8 11, %base
    %last = index i8 %base, 8
    store i8 22, %last
    return obj<9x1> $value
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %value = call obj<9x1> @make()
    %destination = addr @copied
    copyobj 9x1 %value, %destination
    %first = load i8 %destination
    %last_address = index i8 %destination, 8
    %last = load i8 %last_address
    %first_ok = cmp eq i8 %first, 11
    %last_ok = cmp eq i8 %last, 22
    %ok = binary and i64 %first_ok, %last_ok
    %failed = cmp eq i64 %ok, 0
    return i64 %failed
}
