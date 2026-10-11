function @mix(%a : i64, %b : f64, %c : i64, %d : f64) -> i64 {
  block ^entry:
    %bi = convert fptosi i64 f64 %b
    %di = convert fptosi i64 f64 %d
    %sum1 = binary add i64 %a, %c
    %sum2 = binary add i64 %bi, %di
    %sum = binary add i64 %sum1, %sum2
    return i64 %sum
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %lhs = binary add i64 4, 6
    %rhs = binary add i64 8, 12
    %lhsf = convert sitofp f64 i64 %lhs
    %rhsf = convert sitofp f64 i64 %rhs
    %result = call i64 @mix(%lhs, %lhsf, %rhs, %rhsf)
    %ok = cmp eq i64 %result, 50
    return i64 %ok
}
