case _accept_arg_ranges
function encoding
name-source accept_arg_ranges -
let-arg __abi_arg1 type ulong
let-arg __abi_arg2 type ulong
let-type __abi_type2 template helper_inline_ns::v1::pair __abi_arg1 __abi_arg2
let-arg __abi_arg0 type __abi_type2
let-type __abi_type1 template helper_inline_ns::v1::vec __abi_arg0
let-type __abi_type0 ref __abi_type1
param __abi_type0
