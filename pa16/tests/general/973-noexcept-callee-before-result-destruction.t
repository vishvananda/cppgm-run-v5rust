struct K{};template<class T>struct R{~R() noexcept(T::dtor_missing);};template<class T>R<T>make() noexcept(T::callee_missing);static_assert(noexcept(make<K>()),"");
