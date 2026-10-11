let-type T template-param 0
let-type Strip builtin-transform __remove_reference T
let-type Clean builtin-transform __remove_const Strip
let-type First member Clean first_type
let-type FirstClean builtin-transform __remove_const First
let-arg A dependent-value FirstClean int 0
type template Holder A
