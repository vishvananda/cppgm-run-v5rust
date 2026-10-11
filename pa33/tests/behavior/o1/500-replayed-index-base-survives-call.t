global @decoy = {
  i64 0
  i64 0
  i64 0
}

global @holder = {
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

# Leaves every argument register pointing at the decoy: a caller that still
# reads one of them after this call reads the decoy, not its own object.
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

# %field is a constant index the backend replays at its uses; its base is
# %this, whose own last direct use is before the call.
function @publish(%this : ptr, %n : i64) -> void [no_inline=yes] {
  block ^entry:
    %seed = call ptr @fresh()
    %base = index i8 [projection=field] %this, 8
    %field = index i8 [projection=field] %base, 8
    %old = load ptr %field
    call void @clobber(%old)
    %made = call ptr @fresh()
    store ptr %made, %field
    %first = index i8 [projection=field] %this, 0
    store ptr %seed, %first
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
