struct B{virtual void f()throw(int);};template<int N>struct Late:B{void f()noexcept(N);};struct Trigger:B{void f()noexcept(sizeof(Late<0>)!=0);};
