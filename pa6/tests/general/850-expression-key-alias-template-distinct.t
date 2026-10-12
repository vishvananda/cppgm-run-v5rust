// Alias-template declaration identity and exception expression dependencies.
template<class X>using A=X;template<class Y>using B=Y;struct C{template<class T>C(T a,T b)noexcept(sizeof(T)+A);};template<class U>C::C(U x,U y)noexcept(sizeof(U)+B){}
