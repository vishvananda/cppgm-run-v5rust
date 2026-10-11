function encoding
name-source f
function-template-prefix f
let-arg IntArg type int
function-template-arg IntArg
let-type T template-param-subst 0
let-expr SameInt type-trait __is_same T int
let-arg SameIntExpr expression SameInt
let-type IntFlag template Flag SameIntExpr
let-arg IntFlagArg type IntFlag
let-expr SameLong type-trait __is_same T long
let-arg SameLongExpr expression SameLong
let-type LongFlag template Flag SameLongExpr
let-arg LongFlagArg type LongFlag
let-type Return template Pair IntFlagArg LongFlagArg
result Return
