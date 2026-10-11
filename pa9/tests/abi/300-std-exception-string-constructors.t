case _logic_error_string_ctor
function encoding
name-std
name-source logic_error logic_error
name-source  -
terminal constructor-complete
let-arg __abi_arg0 type char
let-arg __abi_arg2 type char
let-type __abi_type3 template std::__1::char_traits __abi_arg2
let-arg __abi_arg1 type __abi_type3
let-arg __abi_arg4 type char
let-type __abi_type4 template std::__1::allocator __abi_arg4
let-arg __abi_arg3 type __abi_type4
let-type __abi_type2 template std::__1::basic_string __abi_arg0 __abi_arg1 __abi_arg3
let-type __abi_type1 const __abi_type2
let-type __abi_type0 ref __abi_type1
param __abi_type0

case _runtime_error_string_ctor
function encoding
name-std
name-source runtime_error runtime_error
name-source  -
terminal constructor-complete
let-arg __abi_arg0 type char
let-arg __abi_arg2 type char
let-type __abi_type3 template std::__1::char_traits __abi_arg2
let-arg __abi_arg1 type __abi_type3
let-arg __abi_arg4 type char
let-type __abi_type4 template std::__1::allocator __abi_arg4
let-arg __abi_arg3 type __abi_type4
let-type __abi_type2 template std::__1::basic_string __abi_arg0 __abi_arg1 __abi_arg3
let-type __abi_type1 const __abi_type2
let-type __abi_type0 ref __abi_type1
param __abi_type0
