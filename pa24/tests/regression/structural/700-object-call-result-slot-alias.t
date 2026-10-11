function @make_value() -> obj<8x8> {
  slot $tmp : obj<8x8>

  block ^entry:
    %a = addr $tmp
    store i64 7, %a
    return obj<8x8> $tmp
}

function @unwrap() -> i64 {
  slot $x : obj<8x8>

  block ^entry:
    %t = call obj<8x8> @make_value()
    %a = addr $x
    copyobj 8x8 %t, %a
    %b = load i64 %a
    return i64 %b
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %ok = call i64 @unwrap()
    return i64 %ok
}
