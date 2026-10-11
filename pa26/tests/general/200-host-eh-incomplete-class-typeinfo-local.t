// Itanium ABI 2.9.5 (__pointer_type_info to an incomplete class is local, with
// __incomplete_mask; the class's own _ZTI belongs to TUs where it is
// complete): this TU, linked first, takes typeid(Derived*) with Derived
// incomplete; a throw of Derived* from u2 must still be caught as Base*
