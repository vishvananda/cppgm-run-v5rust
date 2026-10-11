template<class T>
struct S {
  S();
};

template<class T>
S<T>::S() noexcept {}
