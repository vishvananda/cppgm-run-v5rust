let-arg I type int
let-type T template-param-subst 0
let-expr callee template-id make I
let-expr param0 function-param 0
let-expr deref unary de param0
let-expr result call callee deref
let-type Dep decltype result
function path dep_param_ref_array_helper I
result ref:array:2:char
param ptr:T
param ptr:Dep

case identity
let-arg I type int
let-type T template-param 0
let-expr param0 function-param 0
let-type Bare decltype-id param0
function path identity I
result Bare
param T

case result
let-arg I type int
let-type T template-param 0
let-expr callee source-name selected
let-expr param0 function-param 0
let-expr result call callee param0
let-type Dep decltype result
function path result I
result Dep
param T
