function @choose(%x : i64) -> i64 {
  block ^entry:
    %q = binary div exact i64 %x, 3
    switch %q, ^miss, 0:^a, 1:^a, 2:^b, 3:^a, 4:^b, 5:^a, 6:^b, 7:^a, 8:^b, 9:^a, 10:^b, 11:^a, 12:^b, 13:^a, 14:^b, 15:^a, 16:^b, 17:^a, 18:^b, 19:^a, 20:^b, 21:^a, 22:^b, 23:^a, 24:^b, 25:^a, 26:^b, 27:^a, 28:^b, 29:^a, 30:^b, 31:^a, 32:^b, 33:^a, 34:^b, 35:^a, 36:^b, 37:^a, 38:^b, 39:^a, 40:^b, 41:^a, 42:^b, 43:^a, 44:^b, 45:^a, 46:^b, 47:^a, 48:^b, 49:^a, 50:^b, 51:^a, 52:^b, 53:^a, 54:^b, 55:^a, 56:^b, 57:^a, 58:^b, 59:^a, 60:^b, 61:^a, 62:^b, 63:^a
  block ^a:
    return i64 1
  block ^b:
    return i64 2
  block ^miss:
    return i64 0
}
function @main() -> i64 [role=entry] {
  block ^entry:
    %a = call i64 @choose(0)
    %b = call i64 @choose(6)
    %c = call i64 @choose(189)
    %d = call i64 @choose(192)
    %bad_a = cmp ne i64 %a, 1
    %bad_b = cmp ne i64 %b, 2
    %bad_c = cmp ne i64 %c, 1
    %bad_d = cmp ne i64 %d, 0
    %ab = binary or i64 %bad_a, %bad_b
    %cd = binary or i64 %bad_c, %bad_d
    %bad = binary or i64 %ab, %cd
    return i64 %bad
}
