case ctor_default
function encoding
name-source AbiTagBuffer AbiTagBuffer
name-source  -
terminal constructor-complete

case ctor_copy_tagged
function encoding
name-source AbiTagBuffer AbiTagBuffer
name-source  -
terminal constructor-complete
abi-tag nqe220100
param ref:const:named:AbiTagBuffer

case gptr_const_tagged
function encoding
name-source AbiTagBuffer AbiTagBuffer
name-source gptr -
function-qualifier const
abi-tag nqe220100

case egptr_const_tagged
function encoding
name-source AbiTagBuffer AbiTagBuffer
name-source egptr -
function-qualifier const
abi-tag nqe220100

case setstate_tagged
function encoding
name-source AbiTagBuffer AbiTagBuffer
name-source setstate -
abi-tag nqe220100
param uint
