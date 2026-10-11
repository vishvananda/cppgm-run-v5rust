function encoding
name-source f
function-template-prefix f
let-arg XArg type named:X
function-template-arg XArg
let-type T template-param-subst 0
let-expr Value1 member T 0 value
let-expr One literal 1
let-expr Plus1 binary pl Value1 One
let-type D1 decltype Plus1
let-arg D1Arg type D1
let-expr Value2 member T 0 value
let-expr Two literal 2
let-expr Plus2 binary pl Value2 Two
let-type D2 decltype Plus2
let-arg D2Arg type D2
let-type Return template Pair D1Arg D2Arg
result Return
