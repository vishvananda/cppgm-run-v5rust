global @g : i64 = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    %pg = addr @g
    %ok = cmp ne ptr %pg, 0
    branch %ok, ^good, ^bad

  block ^bad:
    return i64 1

  block ^good:
    return i64 0
}
