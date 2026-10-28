struct S{void f(){void(*p)()noexcept(sizeof(this)==sizeof(void*));}};
