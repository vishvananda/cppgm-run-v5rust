struct K{};template<class T>void f()noexcept(T::first_missing);template<>void f<K>()noexcept(second_missing);
