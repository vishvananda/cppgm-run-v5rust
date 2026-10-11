// Itanium ABI 5.2.3: the key function is the first non-pure virtual function
// declared non-inline in the class, overriders included: for C { int g();
// virtual int key(); } with g overriding B::g it is C::g, so this TU (defining
// C::g) owns C's vtable
