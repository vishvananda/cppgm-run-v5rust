struct K{};template<class T>struct C{virtual void f()noexcept(T::spec_error)=0;};template struct C<K>;
