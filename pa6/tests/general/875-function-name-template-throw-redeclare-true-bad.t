// Declaration name identity, callable facts and exception specification ownership.
template<class T>void f(T)throw(int);template<class U>void f(U)noexcept{}
