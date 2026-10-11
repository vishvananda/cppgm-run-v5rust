function @lhs_false() -> i64 {
  block ^entry:
    return i64 0
}

function @rhs_true() -> i64 {
  block ^entry:
    return i64 1
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %lhs = call i64 @lhs_false()
    branch %lhs, ^true, ^rhs

  block ^rhs:
    %rhsv = call i64 @rhs_true()
    branch %rhsv, ^true, ^false

  block ^true:
    return i64 0

  block ^false:
    return i64 1
}
