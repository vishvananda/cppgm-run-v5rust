declare function @may_throw() -> void [binding=strong]
declare function @sink(%arg0 : ptr) -> void [binding=strong]

function @f(%this : ptr) -> void [binding=strong] {
  slot $this : ptr

  block ^entry:
    store ptr %this, $this
    eh_try ^catch_1
    call void @may_throw()
    eh_end
    return void

  block ^catch_1:
    %t1 = load ptr $this
    call void @sink(%t1)
    return void
}
