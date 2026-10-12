// Alias-template declaration identity and exception expression dependencies.
template<class X>using A=X;struct C{template<class T>C(T a,T b)noexcept(sizeof(T)+A);};template<class U>C::C(U x,U y)noexcept(sizeof(U)+A){}
