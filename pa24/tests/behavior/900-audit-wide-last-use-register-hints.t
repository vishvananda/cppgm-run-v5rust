global @source : i64 = 0

function @multiply() -> i128 {
  block ^entry:
    %scalar = load volatile i64 @source
    %wide = binary mul i128 %scalar, 3
    return i128 %wide
}

function @divide() -> i128 {
  block ^entry:
    %scalar = load volatile i64 @source
    %wide = binary div i128 %scalar, 3
    return i128 %wide
}

function @remainder() -> i128 {
  block ^entry:
    %scalar = load volatile i64 @source
    %wide = binary umod i128 %scalar, 3
    return i128 %wide
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %product = call i128 @multiply()
    %quotient = call i128 @divide()
    %remainder = call i128 @remainder()
    %bad_product = cmp ne i128 %product, 0
    %bad_quotient = cmp ne i128 %quotient, 0
    %bad_remainder = cmp ne i128 %remainder, 0
    %first = binary or i64 %bad_product, %bad_quotient
    %failed = binary or i64 %first, %bad_remainder
    return i64 %failed
}
