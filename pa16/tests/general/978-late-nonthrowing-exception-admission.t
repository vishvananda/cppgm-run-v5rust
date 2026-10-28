struct B{virtual void f()noexcept;};template<int N>struct Late:B{void f()noexcept(N);};void trigger()noexcept(sizeof(Late<0>)!=0);
