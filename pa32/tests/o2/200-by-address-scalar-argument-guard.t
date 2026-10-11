# PA24 README: a scalar temporary passed to a `ptr [pass=by_address]`
# parameter is given temporary storage whose address the call passes.
# Optimizing keeps that meaning: each such call passes the address of
# storage holding the value, never the value itself as a pointer, and no
# load reads through a literal address (lowir.md "pass=by_address").
function @read_i64(%p : ptr [pass=by_address, object_bytes=8]) -> i64
    [binding=internal, no_inline=yes] {
  block ^entry:
    %v = load i64 %p
    return i64 %v
}

function @bump_i64(%p : ptr [pass=by_address, object_bytes=8]) -> i64
    [binding=internal, no_inline=yes] {
  block ^entry:
    %v = load i64 %p
    %w = binary add i64 %v, 10
    store i64 %w, %p
    return i64 %w
}

function @read_i32(%p : ptr [pass=by_address]) -> i32 [binding=internal, no_inline=yes] {
  block ^entry:
    %v = load i32 %p
    return i32 %v
}

function @read_f64(%p : ptr [pass=by_address]) -> f64 [binding=internal, no_inline=yes] {
  block ^entry:
    %v = load f64 %p
    return f64 %v
}

function @difference(%a : ptr [pass=by_address], %b : ptr [pass=by_address]) -> i64
    [binding=internal, no_inline=yes] {
  block ^entry:
    %x = load i64 %a
    %y = load i64 %b
    %d = binary sub i64 %x, %y
    return i64 %d
}

function @plus_one(%p : ptr [pass=by_address]) -> i64 [binding=internal] {
  block ^entry:
    %v = load i64 %p
    %w = binary add i64 %v, 1
    return i64 %w
}

function @known_on_edge(%object : ptr) -> i64 [binding=internal, no_inline=yes] {
  block ^entry:
    %value = load i64 %object
    %is_zero = cmp eq i64 %value, 0
    branch %is_zero, ^known, ^other

  block ^known:
    %r = call i64 @read_i64(%value)
    %s = binary add i64 %r, 20
    return i64 %s

  block ^other:
    return i64 9
}

function @main() -> i64 [role=entry] {
  slot $zero : i64
  block ^entry:
    %five = const i64 5
    %r0 = call i64 @read_i64(%five)
    %r1 = call i64 @bump_i64(%five)
    %r2 = call i64 @read_i64(%five)
    %seven = const i64 7
    %r3 = call i64 @difference(%seven, %five)
    %r4 = call i64 @difference(%five, %five)
    %three = const i32 3
    %r5w = call i32 @read_i32(%three)
    %r5 = convert sext i64 i32 %r5w
    %half = const f64 2.5
    %r6f = call f64 @read_f64(%half)
    %r6d = binary mul f64 %r6f, 2.0
    %r6 = convert fptosi i64 f64 %r6d
    %r7 = call i64 @plus_one(%five)
    %za = addr $zero
    store i64 0, %za
    %r8 = call i64 @known_on_edge(%za)
    %s0 = binary add i64 %r0, %r1
    %s1 = binary add i64 %s0, %r2
    %s2 = binary add i64 %s1, %r3
    %s3 = binary add i64 %s2, %r4
    %s4 = binary add i64 %s3, %r5
    %s5 = binary add i64 %s4, %r6
    %s6 = binary add i64 %s5, %r7
    %s7 = binary add i64 %s6, %r8
    return i64 %s7
}
