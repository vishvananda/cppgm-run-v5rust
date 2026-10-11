let-arg Char type char
let-type Traits template std::char_traits Char
let-arg TraitsArg type Traits
let-type Ostream std-template So yes std::basic_ostream Char TraitsArg
let-type OstreamRef ref Ostream
let-arg ULong type ulong
let-type Value template-param 0
function encoding
name-template basic_ostream std::basic_ostream std::basic_ostream<char,std::char_traits<char>> So yes Char TraitsArg
name-source _M_insert -
function-template-arg ULong
result OstreamRef
param Value
