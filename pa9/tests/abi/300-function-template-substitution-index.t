case _ns__passthrough
function encoding
name-source ns ns
name-source passthrough -
function-template-prefix ns::passthrough
let-arg __abi_arg0 type char
function-template-arg __abi_arg0
let-arg __abi_arg2 type char
let-type __abi_type0 template ns::traits __abi_arg2
let-arg __abi_arg1 type __abi_type0
function-template-arg __abi_arg1
let-type __abi_type3 template-param-subst 0
let-arg __abi_arg3 type __abi_type3
let-type __abi_type4 template-param-subst 1
let-arg __abi_arg4 type __abi_type4
let-type __abi_type2 template ns::stream __abi_arg3 __abi_arg4
let-type __abi_type1 ref __abi_type2
result __abi_type1
let-type __abi_type7 template-param-subst 0
let-arg __abi_arg5 type __abi_type7
let-type __abi_type8 template-param-subst 1
let-arg __abi_arg6 type __abi_type8
let-type __abi_type6 template ns::stream __abi_arg5 __abi_arg6
let-type __abi_type5 ref __abi_type6
param __abi_type5
