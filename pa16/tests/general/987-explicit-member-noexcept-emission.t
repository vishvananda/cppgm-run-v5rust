void g();template<class T>struct C{void f()noexcept(sizeof(T)!=0){g();}};template void C<int>::f();
