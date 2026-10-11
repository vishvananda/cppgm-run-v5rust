function @leaf(%p : ptr) -> obj<8x8> [unwind=no] {
  slot $p : ptr
  slot $retobj__1 : obj<8x8>

  block ^entry:
    store ptr %p, $p
    %t1 = addr $retobj__1
    %t2 = load ptr $p
    store ptr %t2, %t1
    return obj<8x8> $retobj__1
}

function @caller(%p : ptr, %out : ptr) -> void {
  block ^entry:
    eh_try ^dispatch
    %t1 = call obj<8x8> @leaf(%p)
    copyobj 8x8 %t1, %out
    eh_end
    return void

  block ^dispatch:
    resume
}
