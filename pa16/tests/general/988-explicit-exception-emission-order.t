struct K{};template<class T>struct C{void f()noexcept(T::first_missing){}void g()noexcept(T::second_missing){}};template void C<K>::f();template void C<K>::g();
