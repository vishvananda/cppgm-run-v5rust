function @make_value(%input : i64) -> obj<8x8> {
  slot $value : obj<8x8>

  block ^entry:
    %address = addr $value
    store i64 %input, %address
    return obj<8x8> $value
}

function @combine(%left : obj<8x8>, %right : obj<8x8>) -> i64 {
  block ^entry:
    %left_value = load i64 %left
    %right_value = load i64 %right
    %scaled = binary mul i64 %left_value, 100
    %result = binary add i64 %scaled, %right_value
    return i64 %result
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %left = call obj<8x8> @make_value(11)
    %right = call obj<8x8> @make_value(22)
    %combined = call i64 @combine(%left, %right)
    %ok = cmp eq i64 %combined, 1122
    %failed = cmp eq i64 %ok, 0
    return i64 %failed
}
