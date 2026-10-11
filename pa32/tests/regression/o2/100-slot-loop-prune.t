function @main() -> i64 {
  slot $flag : i64

  block ^entry:
    store i64 0, $flag
    jump ^loop

  block ^loop:
    %c = load i64 $flag
    branch %c, ^body, ^exit

  block ^body:
    store i64 1, $flag
    jump ^loop

  block ^exit:
    return i64 7
}
