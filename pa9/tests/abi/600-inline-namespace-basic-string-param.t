let-type Char template-param 0
let-arg Char_arg type Char
let-type Traits template std::char_traits Char_arg
let-arg Traits_arg type Traits
let-type Alloc template std::allocator Char_arg
let-arg Alloc_arg type Alloc
let-type String template std::__cxx11::basic_string Char_arg Traits_arg Alloc_arg
function path std::getline Char_arg
param ref String
