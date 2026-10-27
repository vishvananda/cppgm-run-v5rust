function @f(%input : i64) -> obj<16x8> {
slot $value : obj<16x8>
block ^entry:
 %base = addr $value
 %second = index i64 %base, 1
 store i64 %input, %base
 store i64 7, %second
 %old = load i64 %second
 %new = binary add i64 %old, 1
 store i64 %new, %second
 return obj<16x8> $value
}
function @main() -> i64 [role=entry] {
block ^entry:
 %pair = call obj<16x8> @f(11)
 %first = load i64 %pair
 %second = index i64 %pair, 1
 %high = load i64 %second
 %a = cmp ne i64 %first, 11
 %b = cmp ne i64 %high, 8
 %bad = binary or i1 %a, %b
 return i64 %bad
}
