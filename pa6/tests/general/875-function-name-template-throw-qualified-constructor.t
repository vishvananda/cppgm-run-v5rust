// Declaration name identity, callable facts and exception specification ownership.
struct C{template<class T>C(T)throw(int);};template<class U>C::C(U)throw(long){}
