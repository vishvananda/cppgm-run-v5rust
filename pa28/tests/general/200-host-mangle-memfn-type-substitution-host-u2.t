// Itanium 5.1.5 as g++/clang apply it: in f(int (S::*)(), int (D::*)()) the
// second member function type is mangled FivE again (_Z1fM1SFivEM1DFivE), not
// substituted as S0_; cppgm++ and host objects must agree
