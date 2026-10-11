global @decoy = {
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
}

global @holder = {
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
}

global @fresh_cell = {
  i64 77
}

function @noop(%a : ptr, %b : ptr, %c : ptr, %d : ptr, %e : ptr, %f : ptr) -> void [no_inline=yes] {
  block ^entry:
    return void
}

function @clobber(%p : ptr) -> void [no_inline=yes] {
  block ^entry:
    %d = addr @decoy
    call void @noop(%d, %d, %d, %d, %d, %d)
    return void
}

function @fresh() -> ptr [no_inline=yes] {
  block ^entry:
    %c = addr @fresh_cell
    return ptr %c
}

function @sum8(%a : i64, %b : i64, %c : i64, %d : i64, %e : i64, %f : i64, %g : i64, %h : i64) -> i64 [no_inline=yes] {
  block ^entry:
    %s1 = binary add i64 %a, %b
    %s2 = binary add i64 %s1, %c
    %s3 = binary add i64 %s2, %d
    %s4 = binary add i64 %s3, %e
    %s5 = binary add i64 %s4, %f
    %s6 = binary add i64 %s5, %g
    %s7 = binary add i64 %s6, %h
    return i64 %s7
}

# Eight values stay live across two calls so the parameter is homed in the
# frame; its last direct use is the argument to the first call, while the
# replayed field address is stored through after the second.
function @publish(%this : ptr, %n : i64) -> void [no_inline=yes] {
  block ^entry:
    %f0 = index i8 [projection=field] %this, 24
    %f1 = index i8 [projection=field] %this, 32
    %f2 = index i8 [projection=field] %this, 40
    %f3 = index i8 [projection=field] %this, 48
    %f4 = index i8 [projection=field] %this, 56
    %f5 = index i8 [projection=field] %this, 64
    %f6 = index i8 [projection=field] %this, 72
    %f7 = index i8 [projection=field] %this, 80
    %v0 = load i64 %f0
    %v1 = load i64 %f1
    %v2 = load i64 %f2
    %v3 = load i64 %f3
    %v4 = load i64 %f4
    %v5 = load i64 %f5
    %v6 = load i64 %f6
    %v7 = load i64 %f7
    %seed = call ptr @fresh()
    %base = index i8 [projection=field] %this, 8
    %field = index i8 [projection=field] %base, 8
    %old = load ptr %field
    call void @clobber(%old)
    %made = call ptr @fresh()
    store ptr %made, %field
    %total = call i64 @sum8(%v0, %v1, %v2, %v3, %v4, %v5, %v6, %v7)
    %tslot = index i8 [projection=field] %base, 80
    store i64 %total, %tslot
    return void
}

function @main() -> i32 [role=entry, binding=strong, keep_alias=yes] {
  block ^entry:
    %h = addr @holder
    call void @publish(%h, 3)
    %slot = index i8 [projection=field] %h, 16
    %stored = load ptr %slot
    %expected = addr @fresh_cell
    %bad = cmp ne ptr %stored, %expected
    %d = addr @decoy
    %dslot = index i8 [projection=field] %d, 16
    %dstored = load ptr %dslot
    %leaked = cmp ne ptr %dstored, 0
    %bad64 = convert zext i64 i1 %bad
    %leaked64 = convert zext i64 i1 %leaked
    %sum = binary add i64 %bad64, %leaked64
    %exit = convert trunc i32 i64 %sum
    return i32 %exit
}
