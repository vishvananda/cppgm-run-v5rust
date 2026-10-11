case ctor_complete
function encoding
name-source Cursor Cursor
name-source  -
terminal constructor-complete
param int

case ctor_base
function encoding
name-source Cursor Cursor
name-source  -
terminal constructor-base
param int

case dtor_deleting
function encoding
name-source Cursor Cursor
name-source  -
terminal destructor-deleting

case dtor_complete
function encoding
name-source Cursor Cursor
name-source  -
terminal destructor-complete

case dtor_base
function encoding
name-source Cursor Cursor
name-source  -
terminal destructor-base

case inherited_ctor_complete
function encoding
name-source Derived Derived
name-source  -
terminal constructor-inherited-complete named:Base
param int

case inherited_ctor_base
function encoding
name-source Derived Derived
name-source  -
terminal constructor-inherited-base named:Base
param int
