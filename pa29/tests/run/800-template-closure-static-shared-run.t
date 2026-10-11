// N3485 7.1.2p4, 14.7.1, Itanium C++ ABI 5.1.8 (the closure in a function
// template instance is named Z7counterIiEvENKUlvE_clEv and its static is
// weak): a static local of a lambda in an inline function template instance is
// one object in every unit, under every mix of units built by cppgm++ and by
// the host compiler, also with always_inline
