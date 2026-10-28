template<class...T>struct X{void f()throw(T...);};static_assert(!noexcept(((X<>*)0)->f()),"");
template<class...T>struct B{virtual void f()throw(T...);};struct D:B<>{void f()throw(int);};
