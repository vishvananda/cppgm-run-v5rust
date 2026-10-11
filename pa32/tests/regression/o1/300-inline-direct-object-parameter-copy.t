function @read_object(%value : obj<8x8>) -> i64 {
  slot $local : obj<8x8>

  block ^entry:
    %local_address = addr $local
    copyobj 8x8 %value, %local_address
    %result = load i64 %local_address
    return i64 %result
}

function @main() -> i64 {
  slot $argument : obj<8x8>

  block ^entry:
    %argument_address = addr $argument
    store i64 37, %argument_address
    %result = call i64 @read_object($argument)
    return i64 %result
}
