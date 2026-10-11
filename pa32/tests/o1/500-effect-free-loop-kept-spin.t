function @spin(%flag : i64) -> i64 {
  block ^entry:
    jump ^loop

  block ^loop:
    %c = phi i64 [^entry: %flag, ^loop: 1]
    branch %c, ^loop, ^exit

  block ^exit:
    return i64 7
}
