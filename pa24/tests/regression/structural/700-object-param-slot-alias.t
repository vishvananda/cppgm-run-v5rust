function @make_value() -> obj<8x8> {
  slot $tmp : obj<8x8>

  block ^entry:
    %a = addr $tmp
    store i64 7, %a
    return obj<8x8> $tmp
}

function @unwrap(%x : obj<8x8>) -> i64 {
  slot $x : obj<8x8>

  block ^entry:
    %a = addr $x
    copyobj 8x8 %x, %a
    %b = load i64 %a
    return i64 %b
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %v = call obj<8x8> @make_value()
    %ok = call i64 @unwrap(%v)
    return i64 %ok
}
