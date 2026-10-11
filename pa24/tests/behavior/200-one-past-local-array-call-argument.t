function @consume(%this : ptr [object_bytes=24], %position : obj<8x8>, %first : ptr, %last : ptr, %tag : obj<1x1>) -> i64 [binding=internal, unwind=no] {
  block ^entry:
    %d = binary sub ptr %last, %first
    %dv = copy i64 %d
    return i64 %dv
}

global @probe_result : i64 = zero

function @build(%ret : ptr [pass=indirect_result, object_bytes=24]) -> void [binding=internal, unwind=no] {
  slot $ident : obj<16x1>
  slot $argobj : obj<8x8>
  slot $tag : obj<1x1>
  slot $retobj : obj<8x8>

  block ^entry:
    %t8 = addr $ident
    store u8 127, $ident
    %t9 = index i8 %t8, 1
    store u8 69, %t9
    %t23 = index i8 %t8, 15
    store u8 0, %t23
    %finish_field = index i8 [projection=field] %ret, 8
    %finish = load ptr %finish_field
    %t30 = index i8 %t8, 16
    %argobj_addr = addr $argobj
    %retobj_addr = addr $retobj
    %start = load ptr %ret
    %offset = binary sub ptr %finish, %start
    %pos = index i8 %start, %offset
    %retobj_copy = copy ptr %retobj_addr
    store ptr %pos, %retobj_copy
    copyobj 8x8 $retobj, %argobj_addr
    %r = call i64 @consume(%ret, $argobj, %t8, %t30, $tag)
    store i64 %r, @probe_result
    return void
}

function @main() -> i32 [role=entry, unwind=no] {
  slot $image : obj<24x8>

  block ^entry:
    %image = addr $image
    store ptr nullptr, %image
    %f = index i8 [projection=field] %image, 8
    store ptr nullptr, %f
    %e = index i8 [projection=field] %image, 16
    store ptr nullptr, %e
    call void @build(%image)
    %r = load i64 @probe_result
    %bad = cmp ne i64 %r, 16
    %status = convert zext i32 i1 %bad
    return i32 %status
}
