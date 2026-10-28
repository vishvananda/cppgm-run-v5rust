template<class T>struct A{void f() noexcept(T::missing);};A<int>a;
