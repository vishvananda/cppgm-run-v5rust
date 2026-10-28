template<class T>struct C{void f()noexcept;};template<>void C<int>::f()noexcept;void C<int>::f()noexcept(false);
