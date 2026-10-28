struct B{constexpr explicit operator bool() const{return true;}};template<class T>void f() noexcept(T{});static_assert(noexcept(f<B>()),"");
