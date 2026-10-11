global @buckets = {
  ptr 0
  ptr 0
  ptr 0
  ptr 0
  ptr 0
  ptr 0
  ptr 0
  ptr 0
}
global @table = {
  ptr addr @buckets
  i64 8
  ptr 0
  i64 0
  i64 0
  i64 100
}
global @node = {
  ptr 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
  i64 0
}

function @need_rehash(%policy : ptr, %n_bkt : i64, %n_elt : i64, %n_ins : i64) -> i64 [unwind=no] {
  block ^entry:
    %resize = index i8 %policy, 8
    %limit = load i64 %resize
    %total = binary add i64 %n_elt, %n_ins
    %grow = cmp ugt i64 %total, %limit
    return i64 %grow
}

function @rehash(%this : ptr, %count : i64) -> void [unwind=no] {
  block ^entry:
    %bc = index i8 %this, 8
    store i64 %count, %bc
    return void
}

function @insert(%this : ptr, %bkt : i64, %code : i64, %node : ptr, %n_elt : i64) -> ptr [unwind=no] {
  block ^entry:
    %policy = index i8 %this, 32
    %bc_addr = index i8 %this, 8
    %bc = load i64 %bc_addr
    %ec_addr = index i8 %this, 24
    %ec = load i64 %ec_addr
    %grow = call i64 @need_rehash(%policy, %bc, %ec, %n_elt)
    branch %grow, ^rehash, ^insert

  block ^rehash:
    %doubled = binary mul i64 %bc, 2
    call void @rehash(%this, %doubled)
    %new_bc = load i64 %bc_addr
    %rebkt = binary umod i64 %code, %new_bc
    jump ^insert

  block ^insert:
    %where = phi i64 [^entry: %bkt, ^rehash: %rebkt]
    %hash_addr = index i8 %node, 48
    store i64 %code, %hash_addr
    %buckets = load ptr %this
    %slot = index ptr [projection=array_element] %buckets, %where
    %head = load ptr %slot
    branch %head, ^chain, ^front

  block ^chain:
    %buckets2 = load ptr %this
    %slot2 = index ptr [projection=array_element] %buckets2, %where
    %prev = load ptr %slot2
    %prev_next = load ptr %prev
    store ptr %prev_next, %node
    %buckets3 = load ptr %this
    %slot3 = index ptr [projection=array_element] %buckets3, %where
    %prev2 = load ptr %slot3
    store ptr %node, %prev2
    jump ^done

  block ^front:
    %begin = index i8 %this, 16
    %first = load ptr %begin
    store ptr %first, %node
    store ptr %node, %begin
    %next = load ptr %node
    branch %next, ^rebucket, ^anchor

  block ^rebucket:
    %buckets4 = load ptr %this
    %next_node = load ptr %node
    %next_hash_addr = index i8 %next_node, 48
    %next_bc = load i64 %bc_addr
    %next_hash = load i64 %next_hash_addr
    %next_bkt = binary umod i64 %next_hash, %next_bc
    %next_slot = index ptr [projection=array_element] %buckets4, %next_bkt
    store ptr %node, %next_slot
    jump ^anchor

  block ^anchor:
    %buckets5 = load ptr %this
    %slot5 = index ptr [projection=array_element] %buckets5, %where
    store ptr %begin, %slot5
    jump ^done

  block ^done:
    %count = load i64 %ec_addr
    %count1 = binary add i64 %count, 1
    store i64 %count1, %ec_addr
    return ptr %node
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %table = addr @table
    %node = addr @node
    %inserted = call ptr @insert(%table, 3, 1003, %node, 1)
    %buckets = load ptr %table
    %slot = index ptr [projection=array_element] %buckets, 3
    %anchor = load ptr %slot
    %begin = index i8 %table, 16
    %bad = cmp ne ptr %anchor, %begin
    return i64 %bad
}
