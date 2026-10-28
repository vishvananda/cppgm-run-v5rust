struct S{template<class T>void f() noexcept(sizeof(this)==sizeof(void*));};static_assert(noexcept(((S*)0)->f<int>()),"");
