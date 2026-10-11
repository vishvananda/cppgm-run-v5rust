function encoding
name-source f
function-template-prefix f
let-arg IntArg type int
function-template-arg IntArg
let-arg SecondIntArg type int
function-template-arg SecondIntArg
let-expr P0 function-param 0
let-expr Left cast sc named:ns::C P0
let-expr P1 function-param 1
let-expr Right cast sc named:ns::C P1
let-expr Sum binary pl Left Right
let-type Return decltype Sum
result Return
let-type T template-param-subst 0
param T
let-type U template-param-subst 1
param U
