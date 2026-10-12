// Declaration name identity, callable facts and exception specification ownership.
template<class T>void f(T)noexcept;template<class U>void f(U)throw(int){}
