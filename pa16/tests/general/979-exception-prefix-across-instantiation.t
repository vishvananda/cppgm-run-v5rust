struct K{};struct Base{virtual void f()throw(int);};template<class T>struct S:Base{void f()noexcept(T::nested_missing);};struct O{struct D:Base{void f()noexcept(outer_missing);};S<K>s;};
