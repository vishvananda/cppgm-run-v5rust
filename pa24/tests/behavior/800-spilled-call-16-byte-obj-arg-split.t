function @target(%pair : obj<16x8>, %io : i64) -> i64 {
  slot $pair : obj<16x8>

  block ^entry:
    %p = addr $pair
    copyobj 16x8 %pair, %p
    %lo = load i64 %p
    %hi_ptr = index i8 %p, 8
    %hi = load i64 %hi_ptr
    %s1 = binary add i64 %lo, %hi
    %s2 = binary add i64 %s1, %io
    return i64 %s2
}

function @mid(%fp : ptr, %a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %g : i64) -> i64 {
  slot $pair : obj<16x8>
  slot $force_spill : obj<40x8>

  block ^entry:
    %p = addr $pair
    store i64 1000, %p
    %hi_ptr = index i8 %p, 8
    store i64 2000, %hi_ptr
    %r = call i64 %fp($pair, 111) as (%arg0 : obj<16x8>, %arg1 : i64) -> i64
    return i64 %r
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %fp = addr @target
    %r = call i64 @mid(%fp, 1, 2, 3, 4, 5, 6, 7)
    %ok = cmp eq i64 %r, 3111
    %bad = cmp eq i64 %ok, 0
    return i64 %bad
}
