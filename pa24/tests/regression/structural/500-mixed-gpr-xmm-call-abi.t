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
    %result = call i64 @mix(10, 1.5, 20, 2.5)
    %ok = cmp eq i64 %result, 33
    return i64 %ok
}
