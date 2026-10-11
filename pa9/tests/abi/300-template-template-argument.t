case parameter-template
let-arg TT template-param-template 0
type template Holder TT

case named-template-prefix
let-arg Map template-entity n::M
let-arg Vector template-entity n::V
let-arg Int type int
let-arg Empty pack
let-type DefaultVector template n::V Int Empty
let-arg DefaultVectorArg type DefaultVector
let-type Wrapped template n::W Map Vector DefaultVectorArg
let-arg WrappedArg type Wrapped
typeinfo template O WrappedArg
