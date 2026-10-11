let-type T template-param 0
let-arg T_arg type T
let-type Alloc template ns::Alloc T_arg
let-arg Alloc_arg type Alloc
let-type Owner template ns::Outer T_arg Alloc_arg
type member Owner _Guard_alloc
