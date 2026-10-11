function @make_value() -> obj<8x8> {
  slot $value : obj<8x8>

  block ^entry:
    %address = addr $value
    store i64 77, %address
    return obj<8x8> $value
}

function @read_parameter(%value : obj<8x8>) -> i64 {
  slot $value : obj<8x8>

  block ^entry:
    %local = addr $value
    store i64 99, %local
    %incoming = load i64 %value
    return i64 %incoming
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %value = call obj<8x8> @make_value()
    %read = call i64 @read_parameter(%value)
    %ok = cmp eq i64 %read, 77
    %failed = cmp eq i64 %ok, 0
    %exit = convert trunc i32 i64 %failed
    return i32 %exit
}
