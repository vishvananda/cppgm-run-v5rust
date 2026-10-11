function @leaf(%p : ptr) -> obj<8x8> {
  slot $p : ptr
  slot $retobj__1 : obj<8x8>

  block ^entry:
    store ptr %p, $p
    %t1 = addr $retobj__1
    %t2 = load ptr $p
    store ptr %t2, %t1
    return obj<8x8> $retobj__1
}

function @wrap(%p : ptr) -> obj<8x8> {
  slot $p : ptr
  slot $retobj__1 : obj<8x8>

  block ^entry:
    store ptr %p, $p
    %t1 = addr $retobj__1
    %t2 = load ptr $p
    %t3 = call obj<8x8> @leaf(%t2)
    copyobj 8x8 %t3, %t1
    return obj<8x8> $retobj__1
}

function @caller(%p : ptr, %out : ptr) -> void {
  block ^entry:
    %t1 = call obj<8x8> @wrap(%p)
    copyobj 8x8 %t1, %out
    return void
}
