global @byte : u8 = 0
global @table = {
  zero 16
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %table = addr @table
    %raw = copy i64 %table
    %tagged = binary or i64 %raw, 15
    %mask_bits = const i64 15
    %mask = unary bitnot i64 %mask_bits
    %untagged = binary and i64 %tagged, %mask
    %aligned = cmp eq i64 %untagged, %raw
    branch %aligned, ^ok, ^bad

  block ^bad:
    return i64 1

  block ^ok:
    return i64 0
}
