global @data : obj<16x8> = zero

function @f(%condition : i64) -> i64 {
  block ^entry:
    branch %condition, ^left, ^right

  block ^left:
    %offset = index i8 @data, 8
    jump ^merge

  block ^right:
    jump ^merge

  block ^merge:
    %address = phi ptr [^left: %offset, ^right: 0]
    branch %address, ^yes, ^no

  block ^yes:
    return i64 1

  block ^no:
    return i64 0
}

function @main() -> i32 {
  block ^entry:
    %null = call i64 @f(0)
    %offset = call i64 @f(1)
    %sum = binary add i64 %null, %offset
    %result = binary sub i64 %sum, 1
    return i32 %result
}
