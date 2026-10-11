let-type Identity name identity
let-arg Identity_arg type Identity
let-type Owner template quote_trait Identity_arg
let-arg Fn member-template-entity Owner fn quote_trait<identity>::fn
let-arg Void type void
function encoding
name-template valid_impl valid_impl valid_impl<quote_trait<identity>::fn,void> - no Fn Void
name-source check valid_impl<quote_trait<identity>::fn,void>::check
function-template-arg Fn
param int
