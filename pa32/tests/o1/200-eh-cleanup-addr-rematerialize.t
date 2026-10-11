declare function @may_throw() -> void
declare function @destroy(%object : ptr) -> void [unwind=no]

function @main() -> void {
  slot $object : obj<8x8>

  block ^entry:
    branch 1, ^keep, ^dead

  block ^dead:
    %object_ptr = addr $object
    jump ^keep

  block ^keep:
    eh_try ^cleanup
    call void @may_throw()
    eh_end
    return void

  block ^cleanup:
    call void @destroy(%object_ptr)
    resume
}
